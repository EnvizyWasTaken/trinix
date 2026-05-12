/// Linux x86_64 syscall numbers — stubs for when bash/zsh runs in userspace.
///
/// These are the ABI constants that bash/zsh use. The kernel will need to
/// handle them via an IDT entry for int 0x80 or the `syscall` instruction
/// (via LSTAR MSR). For now they serve as documentation and as compile-time
/// constants for a future syscall dispatcher.

pub const SYS_READ:      u64 = 0;
pub const SYS_WRITE:     u64 = 1;
pub const SYS_OPEN:      u64 = 2;
pub const SYS_CLOSE:     u64 = 3;
pub const SYS_STAT:      u64 = 4;
pub const SYS_FSTAT:     u64 = 5;
pub const SYS_LSTAT:     u64 = 6;
pub const SYS_POLL:      u64 = 7;
pub const SYS_LSEEK:     u64 = 8;
pub const SYS_MMAP:      u64 = 9;
pub const SYS_MPROTECT:  u64 = 10;
pub const SYS_MUNMAP:    u64 = 11;
pub const SYS_BRK:       u64 = 12;
pub const SYS_IOCTL:     u64 = 16;
pub const SYS_DUP:       u64 = 32;
pub const SYS_DUP2:      u64 = 33;
pub const SYS_GETPID:    u64 = 39;
pub const SYS_FORK:      u64 = 57;
pub const SYS_EXECVE:    u64 = 59;
pub const SYS_EXIT:      u64 = 60;
pub const SYS_WAIT4:     u64 = 61;
pub const SYS_GETCWD:    u64 = 79;
pub const SYS_CHDIR:     u64 = 80;
pub const SYS_GETUID:    u64 = 102;
pub const SYS_GETGID:    u64 = 104;
pub const SYS_GETEUID:   u64 = 107;
pub const SYS_GETEGID:   u64 = 108;
pub const SYS_PIPE:      u64 = 22;
pub const SYS_SIGACTION: u64 = 13;
pub const SYS_KILL:      u64 = 62;

/// Stub syscall entry point — replace with a real dispatcher once the IDT
/// and privilege levels are in place. Returns -ENOSYS for everything.
pub fn dispatch(_nr: u64, _a0: u64, _a1: u64, _a2: u64, _a3: u64, _a4: u64, _a5: u64) -> i64 {
    -38 // -ENOSYS
}
