use spin::Mutex;

pub static HISTORY: Mutex<History> = Mutex::new(History::new());

const HIST_CAP:  usize = 50;
const ENTRY_MAX: usize = 256;

pub struct History {
    buf:   [[u8; ENTRY_MAX]; HIST_CAP],
    lens:  [usize; HIST_CAP],
    count: usize,  
    nav:   usize,  
}

impl History {
    pub const fn new() -> Self {
        History {
            buf:   [[0u8; ENTRY_MAX]; HIST_CAP],
            lens:  [0usize; HIST_CAP],
            count: 0,
            nav:   0,
        }
    }

    pub fn push(&mut self, line: &[u8]) {
        if line.is_empty() { return; }
        // Don't add duplicate of the most recent entry
        if self.count > 0 {
            let last_slot = (self.count + HIST_CAP - 1) % HIST_CAP;
            if &self.buf[last_slot][..self.lens[last_slot]] == line {
                self.nav = 0;
                return;
            }
        }
    }

    pub fn reset_nav(&mut self) { self.nav = 0; }

    pub fn up(&mut self) -> Option<&[u8]> {
        if self.nav >= self.count.min(HIST_CAP) { return None; }
        self.nav += 1;
        self.peek()
    }

    pub fn down(&mut self) -> Option<&[u8]> {
        if self.nav == 0 { return None; }
        self.nav -= 1;
        if self.nav == 0 { return Some(b""); }
        self.peek()
    }

    fn peek(&self) -> Option<&[u8]> {
        if self.nav == 0 || self.count == 0 { return None; }
        // nav=1 → most recent, so subtract nav from count
        let idx = (self.count + HIST_CAP - self.nav) % HIST_CAP;
        Some(&self.buf[idx][..self.lens[idx]])
    }

    pub fn count(&self) -> usize { self.count }

    pub fn get(&self, pos: usize) -> Option<&[u8]> {
        if pos >= self.count.min(HIST_CAP) { return None; }
        let oldest = if self.count > HIST_CAP { self.count % HIST_CAP } else { 0 };
        let slot = (oldest + pos) % HIST_CAP;
        Some(&self.buf[slot][..self.lens[slot]])
    }
}
