const BUF_SIZE: usize = 256;

pub struct Shell {
    buf:    [u8; BUF_SIZE],
    len:    usize,
    cursor: usize,
}

impl Shell {
    pub const fn new() -> Self {
        Shell { buf: [0u8; BUF_SIZE], len: 0, cursor: 0 }
    }

    pub fn insert(&mut self, ch: u8) -> bool {
        if self.len >= BUF_SIZE - 1 { return false; }
        self.buf.copy_within(self.cursor..self.len, self.cursor + 1);
        self.buf[self.cursor] = ch;
        self.len    += 1;
        self.cursor += 1;
        true
    }

    pub fn backspace(&mut self) -> bool {
        if self.cursor == 0 { return false; }
        self.cursor -= 1;
        self.buf.copy_within(self.cursor + 1..self.len, self.cursor);
        self.len -= 1;
        true
    }

    pub fn delete(&mut self) -> bool {
        if self.cursor >= self.len { return false; }
        self.buf.copy_within(self.cursor + 1..self.len, self.cursor);
        self.len -= 1;
        true
    }

    pub fn move_left(&mut self)  -> bool { if self.cursor == 0        { false } else { self.cursor -= 1; true } }
    pub fn move_right(&mut self) -> bool { if self.cursor >= self.len { false } else { self.cursor += 1; true } }
    pub fn home(&mut self) { self.cursor = 0; }
    pub fn end(&mut self)  { self.cursor = self.len; }

    pub fn replace_line(&mut self, new_line: &[u8]) {
        let l = new_line.len().min(BUF_SIZE - 1);
        self.buf[..l].copy_from_slice(&new_line[..l]);
        self.len    = l;
        self.cursor = l;
    }

    pub fn cursor(&self) -> usize { self.cursor }
    pub fn line(&self)   -> &[u8] { &self.buf[..self.len] }
    pub fn clear(&mut self)       { self.len = 0; self.cursor = 0; }
}
