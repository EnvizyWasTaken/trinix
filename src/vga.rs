use core::fmt;
use spin::Mutex;
use x86_64::instructions::port::Port;

pub const BUFFER_HEIGHT: usize = 25;
pub const BUFFER_WIDTH:  usize = 80;
const VGA_BASE: usize = 0xb8000;

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
#[allow(dead_code)]
pub enum Color {
    Black      = 0,  Blue       = 1,  Green      = 2,  Cyan      = 3,
    Red        = 4,  Magenta    = 5,  Brown      = 6,  LightGray = 7,
    DarkGray   = 8,  LightBlue  = 9,  LightGreen = 10, LightCyan = 11,
    LightRed   = 12, Pink       = 13, Yellow     = 14, White     = 15,
}

pub const fn color(fg: Color, bg: Color) -> u8 {
    (bg as u8) << 4 | (fg as u8)
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub default:  u8,
    pub dim:      u8,
    pub prompt:   u8,
    pub prompt2:  u8,
    pub banner:   u8,
    pub success:  u8,
    pub error:    u8,
    pub warning:  u8,
    pub info:     u8,
}

pub const MONO: Palette = Palette {
    default:  color(Color::LightGray,  Color::Black),
    dim:      color(Color::DarkGray,   Color::Black),
    prompt:   color(Color::White,      Color::Black),
    prompt2:  color(Color::DarkGray,   Color::Black),
    banner:   color(Color::White,      Color::Black),
    success:  color(Color::LightGreen, Color::Black),
    error:    color(Color::LightRed,   Color::Black),
    warning:  color(Color::Yellow,     Color::Black),
    info:     color(Color::LightGray,  Color::Black),
};

pub const GRUVBOX: Palette = Palette {
    default:  color(Color::LightGray,  Color::Black),
    dim:      color(Color::Brown,      Color::Black),
    prompt:   color(Color::Yellow,     Color::Black),
    prompt2:  color(Color::Brown,      Color::Black),
    banner:   color(Color::Yellow,     Color::Black),
    success:  color(Color::LightGreen, Color::Black),
    error:    color(Color::LightRed,   Color::Black),
    warning:  color(Color::Yellow,     Color::Black),
    info:     color(Color::LightCyan,  Color::Black),
};

pub const DRACULA: Palette = Palette {
    default:  color(Color::LightGray,  Color::Black),
    dim:      color(Color::DarkGray,   Color::Black),
    prompt:   color(Color::Pink,       Color::Black),
    prompt2:  color(Color::Magenta,    Color::Black),
    banner:   color(Color::Pink,       Color::Black),
    success:  color(Color::LightGreen, Color::Black),
    error:    color(Color::LightRed,   Color::Black),
    warning:  color(Color::Yellow,     Color::Black),
    info:     color(Color::LightCyan,  Color::Black),
};

static PALETTE: Mutex<Palette> = Mutex::new(MONO);

pub fn set_palette(p: Palette) { *PALETTE.lock() = p; }
pub fn palette()    -> Palette  { *PALETTE.lock() }

pub struct Writer {
    col:   usize,
    row:   usize,
    color: u8,
}

impl Writer {
    #[inline(always)]
    fn ptr(row: usize, col: usize) -> *mut u16 {
        (VGA_BASE + (row * BUFFER_WIDTH + col) * 2) as *mut u16
    }

    #[inline(always)]
    fn put(&self, row: usize, col: usize, ch: u8) {
        unsafe {
            core::ptr::write_volatile(
                Self::ptr(row, col),
                (self.color as u16) << 8 | ch as u16,
            );
        }
    }

    fn update_cursor(&self) {
        let pos = (self.row * BUFFER_WIDTH + self.col) as u16;
        let mut ctrl: Port<u8> = Port::new(0x3D4);
        let mut data: Port<u8> = Port::new(0x3D5);
        unsafe {
            ctrl.write(0x0F); data.write((pos & 0xFF) as u8);
            ctrl.write(0x0E); data.write((pos >> 8)   as u8);
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.newline(),
            b'\r' => { self.col = 0; }
            b'\t' => {
                let n = 4 - (self.col % 4);
                for _ in 0..n.min(BUFFER_WIDTH - self.col) {
                    self.put(self.row, self.col, b' ');
                    self.col += 1;
                }
            }
            byte => {
                if self.col >= BUFFER_WIDTH { self.newline(); }
                self.put(self.row, self.col, byte);
                self.col += 1;
            }
        }
        self.update_cursor();
    }

    /// Write a raw CP437 byte without ASCII filtering — use for box-drawing chars.
    pub fn write_raw(&mut self, byte: u8) {
        if self.col >= BUFFER_WIDTH { self.newline(); }
        self.put(self.row, self.col, byte);
        self.col += 1;
        self.update_cursor();
    }

    pub fn backspace(&mut self) {
        if self.col > 0 {
            self.col -= 1;
            self.put(self.row, self.col, b' ');
            self.update_cursor();
        }
    }

    fn newline(&mut self) {
        if self.row < BUFFER_HEIGHT - 1 { self.row += 1; } else { self.scroll(); }
        self.col = 0;
    }

    fn scroll(&mut self) {
        unsafe {
            core::ptr::copy(
                Self::ptr(1, 0) as *const u8,
                Self::ptr(0, 0) as *mut u8,
                (BUFFER_HEIGHT - 1) * BUFFER_WIDTH * 2,
            );
        }
        for col in 0..BUFFER_WIDTH { self.put(BUFFER_HEIGHT - 1, col, b' '); }
    }

    pub fn clear(&mut self) {
        for row in 0..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH { self.put(row, col, b' '); }
        }
        self.row = 0; self.col = 0;
        self.update_cursor();
    }

    pub fn set_color(&mut self, c: u8)  { self.color = c; }
    pub fn reset_color(&mut self)        { self.color = palette().default; }

    pub fn cursor_pos(&self) -> (usize, usize) { (self.row, self.col) }

    pub fn set_pos(&mut self, row: usize, col: usize) {
        self.row = row; self.col = col;
        self.update_cursor();
    }

    pub fn clear_to_eol(&mut self) {
        let (row, col) = (self.row, self.col);
        for c in col..BUFFER_WIDTH { self.put(row, c, b' '); }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            match byte {
                0x20..=0x7e | b'\n' | b'\r' | b'\t' => self.write_byte(byte),
                _ => self.write_byte(0xfe),
            }
        }
        Ok(())
    }
}

pub static WRITER: Mutex<Writer> = Mutex::new(Writer { col: 0, row: 0, color: MONO.default });

// start/end are scan-line offsets within a character cell; 14,15 = underline in 8×16
pub fn enable_cursor(start: u8, end: u8) {
    let mut ctrl: Port<u8> = Port::new(0x3D4);
    let mut data: Port<u8> = Port::new(0x3D5);
    unsafe {
        ctrl.write(0x0A); let v = data.read(); data.write((v & 0xC0) | start);
        ctrl.write(0x0B); let v = data.read(); data.write((v & 0xE0) | end);
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    ()            => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}
