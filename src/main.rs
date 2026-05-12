#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

mod commands;
mod env;
mod font;
mod history;
mod keyboard;
mod process;
mod shell;
mod syscall;
mod vga;
mod exfat;
mod ata;
mod config;


use core::panic::PanicInfo;
use keyboard::{Key, Keyboard};
use shell::Shell;
use spin::Mutex;

static KEYBOARD: Mutex<Keyboard> = Mutex::new(Keyboard::new());
static SHELL:    Mutex<Shell>    = Mutex::new(Shell::new());

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    vga::enable_cursor(14, 15);
    font::load(include_bytes!("../assets/fonts/ter-i16n.psf"));

    env::ENV.lock().init();

    vga::WRITER.lock().clear();

    config::load();
    draw_banner();

    match ata::init() {
        Ok(())                         => println!("disk ok"),
        Err(ata::AtaError::NoDevice)   => println!("no disk"),
        Err(e)                         => println!("disk error: {:?}", e),
    }

    match exfat::init() {
        Ok(())                         => println!("exfat mounted"),
        Err(e)                         => println!("exfat error: {:?}", e),
    }

    print_prompt();
    let (mut input_row, mut input_col) = vga::WRITER.lock().cursor_pos();

    loop {
        let key = KEYBOARD.lock().read_key();
        let Some(key) = key else { continue };

        match key {
            Key::Char(b'\n') => {
                println!();
                let mut buf = [0u8; 256];
                let len = {
                    let s = SHELL.lock();
                    let l = s.line().len();
                    buf[..l].copy_from_slice(s.line());
                    l
                };
                SHELL.lock().clear();
                history::HISTORY.lock().push(&buf[..len]);
                history::HISTORY.lock().reset_nav();
                let exit = handle_command(&buf[..len]);
                process::set_exit(exit);
                let code_str: &[u8] = match exit {
                    0 => b"0", 1 => b"1", 2 => b"2", 127 => b"127", _ => b"?",
                };
                env::ENV.lock().set(b"?", code_str);
                print_prompt();
                let pos = vga::WRITER.lock().cursor_pos();
                input_row = pos.0;
                input_col = pos.1;
            }

            Key::ArrowUp => {
                let entry = history::HISTORY.lock().up().map(|e| {
                    let mut tmp = [0u8; 256];
                    let l = e.len().min(255);
                    tmp[..l].copy_from_slice(&e[..l]);
                    (tmp, l)
                });
                if let Some((tmp, l)) = entry {
                    SHELL.lock().replace_line(&tmp[..l]);
                    redraw_input(input_row, input_col);
                }
            }
            Key::ArrowDown => {
                let entry = history::HISTORY.lock().down().map(|e| {
                    let mut tmp = [0u8; 256];
                    let l = e.len().min(255);
                    tmp[..l].copy_from_slice(&e[..l]);
                    (tmp, l)
                });
                if let Some((tmp, l)) = entry {
                    SHELL.lock().replace_line(&tmp[..l]);
                    redraw_input(input_row, input_col);
                }
            }

            Key::Char(0x08) | Key::Char(0x7F) => {
                if SHELL.lock().backspace() { redraw_input(input_row, input_col); }
            }
            Key::Delete => {
                if SHELL.lock().delete() { redraw_input(input_row, input_col); }
            }
            Key::ArrowLeft => {
                if SHELL.lock().move_left() {
                    let cur = SHELL.lock().cursor();
                    vga::WRITER.lock().set_pos(input_row, input_col + cur);
                }
            }
            Key::ArrowRight => {
                if SHELL.lock().move_right() {
                    let cur = SHELL.lock().cursor();
                    vga::WRITER.lock().set_pos(input_row, input_col + cur);
                }
            }
            Key::Home => {
                SHELL.lock().home();
                vga::WRITER.lock().set_pos(input_row, input_col);
            }
            Key::End => {
                let cur = { let mut s = SHELL.lock(); s.end(); s.cursor() };
                vga::WRITER.lock().set_pos(input_row, input_col + cur);
            }

            Key::Char(b'c') | Key::Char(b'C') if KEYBOARD.lock().ctrl() => {
                let p = vga::palette();
                vga::WRITER.lock().set_color(p.dim);
                println!("^C");
                vga::WRITER.lock().reset_color();
                SHELL.lock().clear();
                history::HISTORY.lock().reset_nav();
                process::set_exit(130);
                env::ENV.lock().set(b"?", b"130");
                print_prompt();
                let pos = vga::WRITER.lock().cursor_pos();
                input_row = pos.0;
                input_col = pos.1;
            }

            Key::Char(ch) => {
                if SHELL.lock().insert(ch) { redraw_input(input_row, input_col); }
            }

            _ => {}
        }
    }
}

fn redraw_input(row: usize, start_col: usize) {
    let mut line_buf = [0u8; 256];
    let (len, cursor) = {
        let s = SHELL.lock();
        let l = s.line().len();
        line_buf[..l].copy_from_slice(s.line());
        (l, s.cursor())
    };
    let mut w = vga::WRITER.lock();
    w.set_pos(row, start_col);
    for &b in &line_buf[..len] { w.write_byte(b); }
    w.clear_to_eol();
    w.set_pos(row, start_col + cursor);
}

const TL: u8 = 0xC9;
const TR: u8 = 0xBB;
const BL: u8 = 0xC8;
const BR: u8 = 0xBC;
const H:  u8 = 0xCD;
const V:  u8 = 0xBA;

pub fn draw_banner() {
    let p = vga::palette();
    let mut w = vga::WRITER.lock();
    w.set_color(p.banner);

    w.write_raw(TL);
    for _ in 0..78 { w.write_raw(H); }
    w.write_raw(TR);
    w.write_byte(b'\n');

    let title = b"TRINIX OS  v0.1.0";
    let pad_l = (78 - title.len()) / 2;
    let pad_r = 78 - title.len() - pad_l;
    w.write_raw(V);
    for _ in 0..pad_l { w.write_byte(b' '); }
    for &b in title   { w.write_byte(b); }
    for _ in 0..pad_r { w.write_byte(b' '); }
    w.write_raw(V);
    w.write_byte(b'\n');

    w.write_raw(BL);
    for _ in 0..78 { w.write_raw(H); }
    w.write_raw(BR);
    w.write_byte(b'\n');

    w.set_color(p.dim);
    for &b in b"\n Type 'help' for available commands.\n\n" { w.write_byte(b); }
    w.reset_color();
    drop(w);

    let mut motd_buf = [0u8; 512];
    if let Ok(n) = crate::exfat::read(b"etc/motd", &mut motd_buf) {
        let mut w = vga::WRITER.lock();
        w.set_color(p.info);
        for &b in &motd_buf[..n as usize] {
            if b == b'\n' { w.write_byte(b'\n'); }
            else if b >= 0x20 && b < 0x7f { w.write_byte(b); }
        }
        w.write_byte(b'\n');
        w.reset_color();
    }
}
fn print_prompt() {
    let p = vga::palette();
    {
        let env = env::ENV.lock();
        if let Some(h) = env.get(b"HOSTNAME") {
            if h != b"trinix" {
                vga::WRITER.lock().set_color(p.dim);
                if let Ok(s) = core::str::from_utf8(h) { print!("{}:", s); }
            }
        }
    }
    vga::WRITER.lock().set_color(p.prompt);
    print!("trinix");
    vga::WRITER.lock().set_color(p.prompt2);
    print!("> ");
    vga::WRITER.lock().reset_color();
}

fn handle_command(input: &[u8]) -> i32 {
    let mut expanded = [0u8; 512];
    let elen = commands::expand_vars(input, &mut expanded);
    let input = trim(&expanded[..elen]);
    if input.is_empty() { return 0; }

    let (name, args) = match input.iter().position(|&b| b == b' ') {
        Some(i) => (trim(&input[..i]), trim(&input[i + 1..])),
        None    => (input, &[][..]),
    };

    commands::run(name, args)
}

fn trim(b: &[u8]) -> &[u8] {
    let s = b.iter().position(|&c| c != b' ').unwrap_or(b.len());
    let e = b.iter().rposition(|&c| c != b' ').map(|i| i + 1).unwrap_or(0);
    if s >= e { &[] } else { &b[s..e] }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    vga::WRITER.lock().set_color(vga::color(vga::Color::White, vga::Color::Red));
    println!("\n KERNEL PANIC ");
    if let Some(loc) = info.location() {
        println!("  at {}:{}", loc.file(), loc.line());
    }
    loop {}
}
