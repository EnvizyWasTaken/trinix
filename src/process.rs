use core::sync::atomic::{AtomicI32, Ordering};

static LAST_EXIT: AtomicI32 = AtomicI32::new(0);

pub fn last_exit() -> i32      { LAST_EXIT.load(Ordering::Relaxed) }
pub fn set_exit(code: i32)     { LAST_EXIT.store(code, Ordering::Relaxed); }

pub struct Process {
    pub pid:  u32,
    pub name: [u8; 32],
    pub nlen: usize,
}

impl Process {
    pub const fn kernel() -> Self {
        Process { pid: 0, name: *b"kernel\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0", nlen: 6 }
    }
    pub fn name(&self) -> &[u8] { &self.name[..self.nlen] }
}
