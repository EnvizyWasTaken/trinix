use crate::{env, history, process, vga, println, print};


pub struct Command {
    pub name: &'static str,
    pub desc: &'static str,
}

pub const COMMANDS: &[Command] = &[
    Command { name: "help",     desc: "show this message" },
    Command { name: "echo",     desc: "print arguments" },
    Command { name: "clear",    desc: "clear the screen" },
    Command { name: "version",  desc: "show OS version" },
    Command { name: "uname",    desc: "print system information" },
    Command { name: "whoami",   desc: "print effective user name" },
    Command { name: "hostname", desc: "print or set the hostname" },
    Command { name: "pwd",      desc: "print working directory" },
    Command { name: "env",      desc: "list environment variables" },
    Command { name: "export",   desc: "export variable: export NAME=VALUE" },
    Command { name: "unset",    desc: "unset variable: unset NAME" },
    Command { name: "history",  desc: "show command history" },
    Command { name: "theme",    desc: "switch colour palette: mono  gruvbox  dracula" },
    Command { name: "font",     desc: "switch console font" },
    Command { name: "reboot",   desc: "reboot the system" },
    Command { name: "poweroff", desc: "power off the system" },
    Command { name: "true",     desc: "return exit code 0" },
    Command { name: "false",    desc: "return exit code 1" },
    Command { name: "peek",     desc: "hex dump memory: peek <addr> [len]" },
    Command { name: "ls", desc: "list files and directories" },
    Command { name: "diskinfo", desc: "show disk and filesystem diagnostic info" },
    Command { name: "cat",  desc: "read file: rf <name>" },
    Command { name: "touch",  desc: "new file: nf <name> <content>" },
    Command { name: "cd",  desc: "change directory: cd <dir>" },
    Command { name: "rm",  desc: "delete file or directory: df <name>" },
    Command { name: "mv", desc: "move or rename: mfd <source> <dest>" },
    Command { name: "mkdir", desc: "create directory: crd <name>" },
    Command { name: "edit", desc: "overwrite file contents: edit <name>" },
    Command { name: "panic", desc: "invokes kernel panic" },
    Command { name: "dmesg", desc: "show boot log" },
];

pub fn run(name: &[u8], args: &[u8]) -> i32 {
    match name {
        b"help"     => { cmd_help();              0 }
        b"echo"     => { cmd_echo(args);          0 }
        b"clear"    => { cmd_clear();             0 }
        b"version"  => { cmd_version();           0 }
        b"uname"    => { cmd_uname(args);         0 }
        b"whoami"   => { cmd_whoami();            0 }
        b"hostname" => { cmd_hostname(args);      0 }
        b"pwd"      => { cmd_pwd();               0 }
        b"env"      => { cmd_env();               0 }
        b"export"   => { cmd_export(args)         }
        b"unset"    => { cmd_unset(args);         0 }
        b"history"  => { cmd_history();           0 }
        b"theme"    => { cmd_theme(args);         0 }
        b"font"     => { cmd_font(args);          0 }
        b"reboot"   => cmd_reboot(),
        b"poweroff" => cmd_poweroff(),
        b"true"     =>                            0,
        b"false"    =>                            1,
        b"peek"     => cmd_peek(args),
        b"ls" => { cmd_ldf();                    0 }
        b"diskinfo" => { cmd_diskinfo(); 0 }
        b"cat"  => { cmd_rf(args);  0 }
        b"touch"  => cmd_nf(args),
        b"cd"  => { cmd_cd(args);  0 }
        b"rm"  => cmd_df(args),
        b"mv" => cmd_mfd(args),
        b"mkdir" => cmd_crd(args),
        b"edit" => cmd_edit(args),
        b"panic" => { cmd_panic();                    0 }
        b"dmesg" => { cmd_dmesg(); 0 }
        _ => {
            vga::WRITER.lock().set_color(vga::palette().error);
            if let Ok(s) = core::str::from_utf8(name) {
                println!("error: unknown command '{}'", s);
            }
            vga::WRITER.lock().reset_color();
            127
        }
    }
}

fn cmd_panic() {
    vga::WRITER.lock().set_color(vga::color(vga::Color::White, vga::Color::Red));
    println!("\n KERNEL PANIC ");
    loop {}
}

fn cmd_help() {
    let p = vga::palette();
    vga::WRITER.lock().set_color(p.banner);
    println!("Available commands:");
    vga::WRITER.lock().reset_color();

    for cmd in COMMANDS {
        vga::WRITER.lock().set_color(p.prompt);
        print!("  {:12}", cmd.name);
        vga::WRITER.lock().reset_color();
        println!("  {}", cmd.desc);
    }

    vga::WRITER.lock().set_color(p.dim);
    println!("\n  Themes: mono  gruvbox  dracula");
    print!("  Fonts:  ");
    for &(name, _) in crate::font::FONTS { print!("{}  ", name); }
    println!();
    vga::WRITER.lock().reset_color();
    println!("\n Warning!: Modern fonts like mono and nerd fonts dont work!!!")
}

fn cmd_echo(args: &[u8]) {
    let mut out = [0u8; 512];
    let len = expand_vars(args, &mut out);
    if let Ok(s) = core::str::from_utf8(&out[..len]) {
        println!("{}", s);
    }
}

fn cmd_clear() {
    vga::WRITER.lock().clear();
    crate::draw_banner();
}

fn cmd_version() {
    let mut buf = [0u8; 256];
    match crate::exfat::read(b"etc/os-release", &mut buf) {
        Ok(n) => {
            let p = vga::palette();
            let data = &buf[..n as usize];
            let mut pos = 0usize;
            while pos < data.len() {
                let end = data[pos..].iter().position(|&b| b == b'\n')
                    .map(|i| pos + i).unwrap_or(data.len());
                let line = &data[pos..end];
                pos = end + 1;
                if let Some(eq) = line.iter().position(|&b| b == b'=') {
                    let key = &line[..eq];
                    let val = &line[eq + 1..];
                    vga::WRITER.lock().set_color(p.prompt);
                    if let Ok(k) = core::str::from_utf8(key) { print!("{}: ", k); }
                    vga::WRITER.lock().reset_color();
                    if let Ok(v) = core::str::from_utf8(val) { println!("{}", v); }
                }
            }
        }
        Err(_) => println!("Trinix OS v0.1.0"),
    }
}

fn cmd_uname(args: &[u8]) {
    let all  = args == b"-a";
    let kern = all || args == b"-s" || args.is_empty();
    let rel  = all || args == b"-r";
    let mach = all || args == b"-m";

    let mut buf = [0u8; 256];
    let (name, version) = if crate::exfat::read(b"etc/os-release", &mut buf).is_ok() {
        let mut n = b"Trinix" as &[u8];
        let mut v = b"0.1.0"  as &[u8];
        let mut pos = 0usize;
        while pos < buf.len() {
            let end = buf[pos..].iter().position(|&b| b == b'\n')
                .map(|i| pos + i).unwrap_or(buf.len());
            let line = &buf[pos..end];
            pos = end + 1;
            if line.starts_with(b"NAME=")    { n = &line[5..]; }
            if line.starts_with(b"VERSION=") { v = &line[8..]; }
        }
        (n, v)
    } else {
        (b"Trinix" as &[u8], b"0.1.0" as &[u8])
    };

    let mut first = true;
    let mut put = |s: &[u8]| {
        if !first { print!(" "); }
        if let Ok(s) = core::str::from_utf8(s) { print!("{}", s); }
        first = false;
    };

    if kern { put(name); }
    if rel  { put(version); }
    if mach { put(b"x86_64"); }
    println!();
}

fn cmd_df(args: &[u8]) -> i32 {
    let p = vga::palette();
    if args.is_empty() {
        vga::WRITER.lock().set_color(p.error);
        println!("usage: df <name>");
        vga::WRITER.lock().reset_color();
        return 1;
    }
    match crate::exfat::delete(args) {
        Ok(()) => {
            vga::WRITER.lock().set_color(p.success);
            println!("deleted.");
            vga::WRITER.lock().reset_color();
            0
        }
        Err(e) => {
            vga::WRITER.lock().set_color(p.error);
            println!("error: {:?}", e);
            vga::WRITER.lock().reset_color();
            1
        }
    }
}

fn cmd_crd(args: &[u8]) -> i32 {
    let p = vga::palette();
    if args.is_empty() {
        vga::WRITER.lock().set_color(p.error);
        println!("usage: crd <name>");
        vga::WRITER.lock().reset_color();
        return 1;
    }
    match crate::exfat::create_dir(args) {
        Ok(()) => {
            vga::WRITER.lock().set_color(p.success);
            println!("directory created.");
            vga::WRITER.lock().reset_color();
            0
        }
        Err(e) => {
            vga::WRITER.lock().set_color(p.error);
            println!("error: {:?}", e);
            vga::WRITER.lock().reset_color();
            1
        }
    }
}

fn cmd_mfd(args: &[u8]) -> i32 {
    let p = vga::palette();
    let (src, dst) = match args.iter().position(|&b| b == b' ') {
        Some(i) => (&args[..i], &args[i + 1..]),
        None    => {
            vga::WRITER.lock().set_color(p.error);
            println!("usage: mfd <source> <dest>");
            vga::WRITER.lock().reset_color();
            return 1;
        }
    };
    match crate::exfat::move_file(src, dst) {
        Ok(()) => {
            vga::WRITER.lock().set_color(p.success);
            println!("moved.");
            vga::WRITER.lock().reset_color();
            0
        }
        Err(e) => {
            vga::WRITER.lock().set_color(p.error);
            println!("error: {:?}", e);
            vga::WRITER.lock().reset_color();
            1
        }
    }
}

fn cmd_rf(args: &[u8]) {
    let p = vga::palette();
    let mut buf = [0u8; 8192];
    match crate::exfat::read(args, &mut buf) {
        Err(e) => {
            vga::WRITER.lock().set_color(p.error);
            println!("error: {:?}", e);
            vga::WRITER.lock().reset_color();
        }
        Ok(n) => {
            for &b in &buf[..n as usize] {
                if b == b'\n' { println!(); }
                else if b >= 0x20 && b < 0x7f { print!("{}", b as char); }
            }
            println!();
        }
    }
}

fn cmd_dmesg() {
    let p = vga::palette();
    let mut buf = [0u8; 4096];
    match crate::exfat::read(b"var/log/boot.log", &mut buf) {
        Ok(n) => {
            vga::WRITER.lock().set_color(p.dim);
            for &b in &buf[..n as usize] {
                if b == b'\n' { println!(); }
                else if b >= 0x20 && b < 0x7f { print!("{}", b as char); }
            }
            vga::WRITER.lock().reset_color();
        }
        Err(_) => {
            vga::WRITER.lock().set_color(p.error);
            println!("no boot log found");
            vga::WRITER.lock().reset_color();
        }
    }
}

fn cmd_nf(args: &[u8]) -> i32 {
    let p = vga::palette();
    let (name, content) = match args.iter().position(|&b| b == b' ') {
        Some(i) => (&args[..i], &args[i + 1..]),
        None    => {
            vga::WRITER.lock().set_color(p.error);
            println!("usage: nf <name> <content>");
            vga::WRITER.lock().reset_color();
            return 1;
        }
    };
    match crate::exfat::create(name, content) {
        Ok(()) => {
            vga::WRITER.lock().set_color(p.success);
            println!("file created.");
            vga::WRITER.lock().reset_color();
            0
        }
        Err(e) => {
            vga::WRITER.lock().set_color(p.error);
            println!("error: {:?}", e);
            vga::WRITER.lock().reset_color();
            1
        }
    }
}

fn cmd_cd(args: &[u8]) {
    let p    = vga::palette();
    let dest = if args.is_empty() { b"/" as &[u8] } else { args };
    match crate::exfat::chdir(dest) {
        Ok(()) => {
            env::ENV.lock().set(b"PWD", dest);
        }
        Err(crate::exfat::ExFatError::NotADirectory) => {
            vga::WRITER.lock().set_color(p.error);
            println!("cd: not a directory");
            vga::WRITER.lock().reset_color();
        }
        Err(crate::exfat::ExFatError::FileNotFound) => {
            vga::WRITER.lock().set_color(p.error);
            println!("cd: directory not found");
            vga::WRITER.lock().reset_color();
        }
        Err(e) => {
            vga::WRITER.lock().set_color(p.error);
            println!("cd: error {:?}", e);
            vga::WRITER.lock().reset_color();
        }
    }
}

fn cmd_whoami() {
    let env = env::ENV.lock();
    if let Some(u) = env.get(b"USER") {
        if let Ok(s) = core::str::from_utf8(u) { println!("{}", s); }
    }
}

fn cmd_hostname(args: &[u8]) {
    if args.is_empty() {
        let env = env::ENV.lock();
        if let Some(h) = env.get(b"HOSTNAME") {
            if let Ok(s) = core::str::from_utf8(h) { println!("{}", s); }
        }
    } else {
        env::ENV.lock().set(b"HOSTNAME", args);
        let p = vga::palette();
        vga::WRITER.lock().set_color(p.success);
        println!("hostname set.");
        vga::WRITER.lock().reset_color();
    }
}

fn cmd_pwd() {
    let env = env::ENV.lock();
    if let Some(d) = env.get(b"PWD") {
        if let Ok(s) = core::str::from_utf8(d) { println!("{}", s); }
    }
}

fn cmd_env() {
    let env = env::ENV.lock();
    for (k, v) in env.iter() {
        if let (Ok(ks), Ok(vs)) = (core::str::from_utf8(k), core::str::from_utf8(v)) {
            println!("{}={}", ks, vs);
        }
    }
}

fn cmd_ldf() {
    let p = vga::palette();
    let mut entries = [const { crate::exfat::DirEntry::empty() }; 64];

    let n = match crate::exfat::ls(&mut entries) {
        Ok(n)  => n,
        Err(e) => {
            vga::WRITER.lock().set_color(p.error);
            println!("error: {:?}", e);
            vga::WRITER.lock().reset_color();
            return;
        }
    };

    vga::WRITER.lock().set_color(p.banner);
    println!("{:<40} {:>12}  {}", "name", "size", "type");
    vga::WRITER.lock().reset_color();
    vga::WRITER.lock().set_color(p.dim);
    println!("──────────────────────────────────────────────────────────");
    vga::WRITER.lock().reset_color();

    for e in &entries[..n] {
        let Ok(name) = core::str::from_utf8(e.name_bytes()) else { continue };
        let mut w = vga::WRITER.lock();
        if e.is_dir {
            w.set_color(p.info);
            drop(w);
            println!("{:<40} {:>12}  dir", name, "-");
        } else {
            w.set_color(p.default);
            drop(w);
            println!("{:<40} {:>12}  file", name, e.size);
        }
        vga::WRITER.lock().reset_color();
    }
}

// `export NAME=VALUE` sets a variable; `export NAME` prints its current value.
fn cmd_export(args: &[u8]) -> i32 {
    if let Some(eq) = args.iter().position(|&b| b == b'=') {
        env::ENV.lock().set(&args[..eq], &args[eq + 1..]);
        0
    } else if !args.is_empty() {
        let env = env::ENV.lock();
        if let Some(v) = env.get(args) {
            if let (Ok(k), Ok(vs)) = (core::str::from_utf8(args), core::str::from_utf8(v)) {
                println!("{}={}", k, vs);
            }
        } else {
            let p = vga::palette();
            vga::WRITER.lock().set_color(p.warning);
            if let Ok(k) = core::str::from_utf8(args) {
                println!("warning: {} not set", k);
            }
            vga::WRITER.lock().reset_color();
        }
        0
    } else {
        cmd_env(); 0
    }
}

fn cmd_unset(args: &[u8]) {
    env::ENV.lock().unset(args);
}

fn cmd_history() {
    let hist = history::HISTORY.lock();
    let total = hist.count();
    let start = if total > 50 { total - 50 } else { 0 };
    for i in 0..(total - start) {
        if let Some(line) = hist.get(i) {
            let num = start + i + 1;
            if let Ok(s) = core::str::from_utf8(line) {
                println!("{:4}  {}", num, s);
            }
        }
    }
}

pub fn cmd_theme(arg: &[u8]) {
    let p = match arg {
        b"mono"    => vga::MONO,
        b"gruvbox" => vga::GRUVBOX,
        b"dracula" => vga::DRACULA,
        _ => {
            let cur = vga::palette();
            vga::WRITER.lock().set_color(cur.error);
            println!("unknown theme — try: mono  gruvbox  dracula");
            vga::WRITER.lock().reset_color();
            return;
        }
    };
    vga::set_palette(p);
    vga::WRITER.lock().clear();
    crate::draw_banner();
    vga::WRITER.lock().set_color(vga::palette().success);
    println!("theme applied.");
    vga::WRITER.lock().reset_color();
}

pub fn cmd_font(arg: &[u8]) {
    let p = vga::palette();
    if let Ok(name) = core::str::from_utf8(arg) {
        for &(font_name, font_data) in crate::font::FONTS {
            if font_name == name {
                if crate::font::load(font_data) {
                    vga::WRITER.lock().set_color(p.success);
                    println!("font loaded.");
                } else {
                    vga::WRITER.lock().set_color(p.error);
                    println!("error: could not parse font.");
                }
                vga::WRITER.lock().reset_color();
                return;
            }
        }
    }
    vga::WRITER.lock().set_color(p.error);
    println!("unknown font — type 'help' to see available fonts.");
    vga::WRITER.lock().reset_color();
}

fn cmd_reboot() -> ! {
    // PS/2 controller pulse reset line (port 0x64, command 0xFE)
    use x86_64::instructions::port::Port;
    let mut p: Port<u8> = Port::new(0x64);
    unsafe { p.write(0xFE); }
    loop { x86_64::instructions::hlt(); }
}

fn cmd_poweroff() -> ! {
    // ACPI S5 sleep via QEMU/Bochs ioport
    use x86_64::instructions::port::Port;
    let mut p: Port<u16> = Port::new(0x604);
    unsafe { p.write(0x2000); }
    loop { x86_64::instructions::hlt(); }
}

/// Expand `$NAME`, `$?`, and `$$` references in `src` into `dst`.
/// Returns the number of bytes written.
pub fn expand_vars(src: &[u8], dst: &mut [u8]) -> usize {
    let mut si = 0;
    let mut di = 0;

    while si < src.len() && di < dst.len() {
        if src[si] == b'$' && si + 1 < src.len() {
            si += 1;
            if src[si] == b'?' {
                si += 1;
                let s = fmt_exit(process::last_exit());
                let l = s.len().min(dst.len() - di);
                dst[di..di + l].copy_from_slice(&s.as_bytes()[..l]);
                di += l;
                continue;
            }
            if src[si] == b'$' {
                // $$ is always 0 — no real PID in kernel context
                si += 1;
                dst[di] = b'0'; di += 1;
                continue;
            }
            let start = si;
            while si < src.len() && (src[si].is_ascii_alphanumeric() || src[si] == b'_') {
                si += 1;
            }
            if let Some(val) = env::ENV.lock().get(&src[start..si]) {
                let l = val.len().min(dst.len() - di);
                dst[di..di + l].copy_from_slice(&val[..l]);
                di += l;
            }
        } else {
            dst[di] = src[si];
            di += 1;
            si += 1;
        }
    }
    di
}

fn cmd_edit(args: &[u8]) -> i32 {
    let p = vga::palette();

    if args.is_empty() {
        vga::WRITER.lock().set_color(p.error);
        println!("usage: edit <filename>");
        vga::WRITER.lock().reset_color();
        return 1;
    }

    let mut buf = [0u8; 8192];
    match crate::exfat::read(args, &mut buf) {
        Ok(n) => {
            vga::WRITER.lock().set_color(p.dim);
            println!("current content:");
            vga::WRITER.lock().reset_color();
            for &b in &buf[..n as usize] {
                if b == b'\n' { println!(); }
                else if b >= 0x20 && b < 0x7f { print!("{}", b as char); }
            }
            println!();
        }
        Err(crate::exfat::ExFatError::FileNotFound) => {
            vga::WRITER.lock().set_color(p.dim);
            println!("new file.");
            vga::WRITER.lock().reset_color();
        }
        Err(e) => {
            vga::WRITER.lock().set_color(p.error);
            println!("error: {:?}", e);
            vga::WRITER.lock().reset_color();
            return 1;
        }
    }

    vga::WRITER.lock().set_color(p.prompt);
    print!("new content > ");
    vga::WRITER.lock().reset_color();

    let content = read_line();

    match crate::exfat::delete(args) { _ => {} }
    match crate::exfat::create(args, &content[..]) {
        Ok(()) => {
            vga::WRITER.lock().set_color(p.success);
            println!("saved.");
            vga::WRITER.lock().reset_color();
            0
        }
        Err(e) => {
            vga::WRITER.lock().set_color(p.error);
            println!("error: {:?}", e);
            vga::WRITER.lock().reset_color();
            1
        }
    }
}

fn read_line() -> [u8; 256] {
    let mut buf = [0u8; 256];
    let mut len = 0usize;
    loop {
        let key = crate::KEYBOARD.lock().read_key();
        let Some(key) = key else { continue };
        match key {
            crate::keyboard::Key::Char(b'\n') => {
                println!();
                break;
            }
            crate::keyboard::Key::Char(0x08) | crate::keyboard::Key::Char(0x7F) => {
                if len > 0 {
                    len -= 1;
                    crate::vga::WRITER.lock().backspace();
                }
            }
            crate::keyboard::Key::Char(ch) if ch >= 0x20 => {
                if len < 255 {
                    buf[len] = ch;
                    len += 1;
                    print!("{}", ch as char);
                }
            }
            _ => {}
        }
    }
    buf
}

fn cmd_diskinfo() {
    let p = vga::palette();

    for drive_sel in [0xE0u8, 0xF0u8] {
        use x86_64::instructions::port::Port;
        unsafe {
            Port::<u8>::new(0x1F6).write(drive_sel);
            // io delay
            for _ in 0..4 { Port::<u8>::new(0x3F6).read(); }
        }

        let mut buf = [0u8; 512];
        match crate::ata::read(0, &mut buf) {
            Err(e) => {
                vga::WRITER.lock().set_color(p.error);
                println!("drive {:02x} ata error: {:?}", drive_sel, e);
                vga::WRITER.lock().reset_color();
                continue;
            }
            Ok(()) => {}
        }

        vga::WRITER.lock().set_color(p.banner);
        println!("drive {:02x} sector 0 ok", drive_sel);
        vga::WRITER.lock().reset_color();
        println!("  OEM: {:?}", core::str::from_utf8(&buf[3..11]).unwrap_or("?"));
        print!("  raw: ");
        for b in &buf[3..11] { print!("{:02x} ", b); }
        println!();
    }
}

fn cmd_peek(args: &[u8]) -> i32 {
    let p = vga::palette();
    let (addr_tok, rest) = next_token(args);

    if addr_tok.is_empty() {
        vga::WRITER.lock().set_color(p.error);
        println!("usage: peek <hex_addr> [len]");
        vga::WRITER.lock().reset_color();
        return 1;
    }

    let addr = match parse_hex(addr_tok) {
        Some(a) => a as usize,
        None => {
            vga::WRITER.lock().set_color(p.error);
            println!("error: invalid address");
            vga::WRITER.lock().reset_color();
            return 1;
        }
    };

    let (len_tok, _) = next_token(rest);
    let len: usize = if len_tok.is_empty() {
        16
    } else {
        match parse_dec(len_tok) {
            Some(n) => n.min(256),
            None => {
                vga::WRITER.lock().set_color(p.error);
                println!("error: invalid length");
                vga::WRITER.lock().reset_color();
                return 1;
            }
        }
    };

    if !is_safe_peek_addr(addr, len) {
        vga::WRITER.lock().set_color(p.error);
        println!("error: address range is restricted");
        vga::WRITER.lock().reset_color();
        return 1;
    }

    let mut offset = 0usize;
    while offset < len {
        let row_len = (len - offset).min(16);
        let mut row = [0u8; 16];
        for i in 0..row_len {
            row[i] = unsafe { core::ptr::read_volatile((addr + offset + i) as *const u8) };
        }

        vga::WRITER.lock().set_color(p.dim);
        print!("{:08x}  ", addr + offset);

        vga::WRITER.lock().set_color(p.default);
        for i in 0..16usize {
            if i == 8 { print!(" "); }
            if i < row_len { print!("{:02x} ", row[i]); }
            else           { print!("   "); }
        }

        vga::WRITER.lock().set_color(p.dim);
        print!("|");
        {
            let mut w = vga::WRITER.lock();
            w.set_color(p.info);
            for i in 0..16 {
                let b = if i < row_len {
                    let raw = row[i];
                    if raw >= 0x20 && raw < 0x7f { raw } else { b'.' }
                } else {
                    b' '
                };
                w.write_byte(b);
            }
        }
        vga::WRITER.lock().set_color(p.dim);
        println!("|");

        offset += row_len;
    }

    vga::WRITER.lock().reset_color();
    0
}

fn next_token(s: &[u8]) -> (&[u8], &[u8]) {
    let s = {
        let i = s.iter().position(|&b| b != b' ' && b != b'\t').unwrap_or(s.len());
        &s[i..]
    };
    match s.iter().position(|&b| b == b' ' || b == b'\t') {
        Some(i) => (&s[..i], &s[i..]),
        None    => (s, &[]),
    }
}

fn is_safe_peek_addr(addr: usize, len: usize) -> bool {
    let end = addr.saturating_add(len);

    // Carve out VGA text buffer before the legacy MMIO block
    let vga_start = 0xB_8000usize;
    let vga_end   = 0xB_A000usize;
    let in_vga    = addr >= vga_start && end <= vga_end;

    if addr < 0x1000                                      { return false; }
    if !in_vga && addr >= 0xA_0000 && addr < 0x10_0000   { return false; }
    if addr >= 0xE000_0000                                { return false; }
    if addr >= 0xFEE0_0000 && addr < 0xFEF0_0000         { return false; }
    if addr >= 0xFEC0_0000 && addr < 0xFED0_0000         { return false; }

    if in_vga                                             { return true; }
    if addr >= 0x10_0000 && end <= 0x40_0000             { return true; }
    if addr >= 0x400    && end <= 0x9_0000               { return true; }

    false
}

fn parse_hex(s: &[u8]) -> Option<u64> {
    let s = if s.starts_with(b"0x") || s.starts_with(b"0X") { &s[2..] } else { s };
    if s.is_empty() { return None; }
    let mut val: u64 = 0;
    for &b in s {
        let digit = match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            b'A'..=b'F' => b - b'A' + 10,
            _ => return None,
        };
        val = val.checked_mul(16)?.checked_add(digit as u64)?;
    }
    Some(val)
}

fn parse_dec(s: &[u8]) -> Option<usize> {
    let s = {
        let i = s.iter().position(|&b| b != b' ' && b != b'\t').unwrap_or(s.len());
        &s[i..]
    };
    if s.is_empty() { return None; }
    let mut val: usize = 0;
    for &b in s {
        let digit = match b {
            b'0'..=b'9' => b - b'0',
            _ => return None,
        };
        val = val.checked_mul(10)?.checked_add(digit as usize)?;
    }
    Some(val)
}

// Only the exit codes the shell can actually produce — anything else is unexpanded.
fn fmt_exit(n: i32) -> &'static str {
    match n {
        0   => "0",
        1   => "1",
        2   => "2",
        126 => "126",
        127 => "127",
        130 => "130",
        _   => "?",
    }
}
