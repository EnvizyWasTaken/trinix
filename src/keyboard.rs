use x86_64::instructions::port::Port;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Key {
    Char(u8),
    ArrowUp, ArrowDown, ArrowLeft, ArrowRight,
    Home, End, Insert, Delete,
    PageUp, PageDown,
    F(u8),
    CtrlPressed, CtrlReleased,
    AltPressed,  AltReleased,
}

// Scancode set 1 — unshifted
static MAP: [u8; 89] = [
//  0x00   0x01    0x02   0x03   0x04   0x05   0x06   0x07
    0,     0x1b,  b'1',  b'2',  b'3',  b'4',  b'5',  b'6',
//  0x08   0x09   0x0a   0x0b   0x0c   0x0d   0x0e   0x0f
    b'7',  b'8',  b'9',  b'0',  b'-',  b'=',  0x08,  b'\t',
//  0x10   0x11   0x12   0x13   0x14   0x15   0x16   0x17
    b'q',  b'w',  b'e',  b'r',  b't',  b'y',  b'u',  b'i',
//  0x18   0x19   0x1a   0x1b   0x1c   0x1d   0x1e   0x1f
    b'o',  b'p',  b'[',  b']',  b'\n', 0,     b'a',  b's',
//  0x20   0x21   0x22   0x23   0x24   0x25   0x26   0x27
    b'd',  b'f',  b'g',  b'h',  b'j',  b'k',  b'l',  b';',
//  0x28   0x29   0x2a   0x2b   0x2c   0x2d   0x2e   0x2f
    b'\'', b'`',  0,     b'\\', b'z',  b'x',  b'c',  b'v',
//  0x30   0x31   0x32   0x33   0x34   0x35   0x36   0x37
    b'b',  b'n',  b'm',  b',',  b'.',  b'/',  0,     b'*',
//  0x38   0x39   0x3a   0x3b   0x3c   0x3d   0x3e   0x3f
    0,     b' ',  0,     0,     0,     0,     0,     0,
//  0x40   0x41   0x42   0x43   0x44   0x45   0x46   0x47
    0,     0,     0,     0,     0,     0,     0,     b'7',
//  0x48   0x49   0x4a   0x4b   0x4c   0x4d   0x4e   0x4f
    b'8',  b'9',  b'-',  b'4',  b'5',  b'6',  b'+',  b'1',
//  0x50   0x51   0x52   0x53   0x54   0x55   0x56   0x57   0x58
    b'2',  b'3',  b'0',  b'.',  0,     0,     0,     0,     0,
];

// Scancode set 1 — shifted
static MAP_SHIFT: [u8; 89] = [
    0,     0x1b,  b'!',  b'@',  b'#',  b'$',  b'%',  b'^',
    b'&',  b'*',  b'(',  b')',  b'_',  b'+',  0x08,  b'\t',
    b'Q',  b'W',  b'E',  b'R',  b'T',  b'Y',  b'U',  b'I',
    b'O',  b'P',  b'{',  b'}',  b'\n', 0,     b'A',  b'S',
    b'D',  b'F',  b'G',  b'H',  b'J',  b'K',  b'L',  b':',
    b'"',  b'~',  0,     b'|',  b'Z',  b'X',  b'C',  b'V',
    b'B',  b'N',  b'M',  b'<',  b'>',  b'?',  0,     b'*',
    0,     b' ',  0,     0,     0,     0,     0,     0,
    0,     0,     0,     0,     0,     0,     0,     b'7',
    b'8',  b'9',  b'-',  b'4',  b'5',  b'6',  b'+',  b'1',
    b'2',  b'3',  b'0',  b'.',  0,     0,     0,     0,     0,
];

fn fkey(sc: u8) -> u8 {
    match sc {
        0x3B => 1,  0x3C => 2,  0x3D => 3,  0x3E => 4,
        0x3F => 5,  0x40 => 6,  0x41 => 7,  0x42 => 8,
        0x43 => 9,  0x44 => 10, 0x57 => 11, 0x58 => 12,
        _ => 0,
    }
}

// E0-prefixed extended make-codes (arrow keys, ins/del, etc.)
fn extended(sc: u8) -> Option<Key> {
    match sc {
        0x47 => Some(Key::Home),
        0x48 => Some(Key::ArrowUp),
        0x49 => Some(Key::PageUp),
        0x4B => Some(Key::ArrowLeft),
        0x4D => Some(Key::ArrowRight),
        0x4F => Some(Key::End),
        0x50 => Some(Key::ArrowDown),
        0x51 => Some(Key::PageDown),
        0x52 => Some(Key::Insert),
        0x53 => Some(Key::Delete),
        0x1D => Some(Key::CtrlPressed),
        0x38 => Some(Key::AltPressed),
        0x9D => Some(Key::CtrlReleased),
        0xB8 => Some(Key::AltReleased),
        _ => None,
    }
}

pub struct Keyboard {
    shift:    bool,
    caps:     bool,
    ctrl:     bool,
    extended: bool,
}

impl Keyboard {
    pub const fn new() -> Self {
        Keyboard { shift: false, caps: false, ctrl: false, extended: false }
    }

    pub fn ctrl(&self) -> bool { self.ctrl }

    pub fn read_key(&mut self) -> Option<Key> {
        let mut status: Port<u8> = Port::new(0x64);
        let mut data:   Port<u8> = Port::new(0x60);

        if unsafe { status.read() } & 0x01 == 0 {
            return None;
        }
        let sc = unsafe { data.read() };

        if sc == 0xE0 {
            self.extended = true;
            return None;
        }

        if self.extended {
            self.extended = false;
            return extended(sc);
        }

        match sc {
            0x2A | 0x36 => { self.shift = true;       None }
            0xAA | 0xB6 => { self.shift = false;      None }
            0x3A        => { self.caps = !self.caps;   None }
            0x1D        => { self.ctrl = true;  Some(Key::CtrlPressed)  }
            0x9D        => { self.ctrl = false; Some(Key::CtrlReleased) }
            0x38        => Some(Key::AltPressed),
            0xB8        => Some(Key::AltReleased),
            0x80..      => None, // key-release codes
            sc => {
                let f = fkey(sc);
                if f > 0 { return Some(Key::F(f)); }

                let i = sc as usize;
                if i >= MAP.len() { return None; }

                let base = MAP[i];
                if base == 0 { return None; }

                let is_letter = base >= b'a' && base <= b'z';
                let ch = if is_letter {
                    if self.caps ^ self.shift { MAP_SHIFT[i] } else { base }
                } else {
                    if self.shift { MAP_SHIFT[i] } else { base }
                };
                Some(Key::Char(ch))
            }
        }
    }
}
