use spin::Mutex;

pub static ENV: Mutex<Env> = Mutex::new(Env::new());

const MAX_VARS: usize = 64;
const MAX_KEY:  usize = 32;
const MAX_VAL:  usize = 128;

pub struct Env {
    keys:   [[u8; MAX_KEY]; MAX_VARS],
    vals:   [[u8; MAX_VAL]; MAX_VARS],
    klens:  [usize; MAX_VARS],
    vlens:  [usize; MAX_VARS],
    count:  usize,
}

impl Env {
    pub const fn new() -> Self {
        Env {
            keys:  [[0u8; MAX_KEY]; MAX_VARS],
            vals:  [[0u8; MAX_VAL]; MAX_VARS],
            klens: [0usize; MAX_VARS],
            vlens: [0usize; MAX_VARS],
            count: 0,
        }
    }

    pub fn init(&mut self) {
        self.set(b"SHELL",    b"trinix");
        self.set(b"TERM",     b"vt100");
        self.set(b"HOME",     b"/");
        self.set(b"USER",     b"root");
        self.set(b"HOSTNAME", b"trinix");
        self.set(b"PATH",     b"/bin:/usr/bin");
        self.set(b"PWD",      b"/");
        self.set(b"?",        b"0");
    }

    pub fn set(&mut self, key: &[u8], val: &[u8]) {
        for i in 0..self.count {
            if &self.keys[i][..self.klens[i]] == key {
                let l = val.len().min(MAX_VAL);
                self.vals[i][..l].copy_from_slice(&val[..l]);
                self.vlens[i] = l;
                return;
            }
        }
        if self.count < MAX_VARS {
            let kl = key.len().min(MAX_KEY);
            let vl = val.len().min(MAX_VAL);
            self.keys[self.count][..kl].copy_from_slice(&key[..kl]);
            self.vals[self.count][..vl].copy_from_slice(&val[..vl]);
            self.klens[self.count] = kl;
            self.vlens[self.count] = vl;
            self.count += 1;
        }
    }

    pub fn get(&self, key: &[u8]) -> Option<&[u8]> {
        for i in 0..self.count {
            if &self.keys[i][..self.klens[i]] == key {
                return Some(&self.vals[i][..self.vlens[i]]);
            }
        }
        None
    }

    pub fn unset(&mut self, key: &[u8]) {
        for i in 0..self.count {
            if &self.keys[i][..self.klens[i]] == key {
                if i + 1 < self.count {
                    self.keys.copy_within(i + 1..self.count, i);
                    self.vals.copy_within(i + 1..self.count, i);
                    self.klens.copy_within(i + 1..self.count, i);
                    self.vlens.copy_within(i + 1..self.count, i);
                }
                self.count -= 1;
                return;
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&[u8], &[u8])> {
        (0..self.count).map(move |i| {
            (&self.keys[i][..self.klens[i]], &self.vals[i][..self.vlens[i]])
        })
    }
}
