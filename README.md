# Trinix OS

A bare-metal x86_64 operating system kernel written in Rust.

## Features
- VGA text output with multiple color themes (mono, gruvbox, dracula)
- Interactive shell with history, line editing, and environment variables
- PSF font support
- CFS preemptive scheduler
- ATA PIO disk driver
- exFAT filesystem (read/write) with full file and directory management
- Boot configuration via `/etc/trinix.conf`

## Building

Requires Rust nightly and `bootimage`:

```bash
cargo install bootimage
cargo bootimage
```

## Running

```bash
qemu-system-x86_64 \
  -drive format=raw,file=target/x86_64-trinix/debug/bootimage-trinix-os.bin \
  -drive format=raw,file=disk.img,if=ide
```

## Shell Commands

| Command | Description |
|---------|-------------|
| `ldf` | list files and directories |
| `rf <name>` | read file |
| `nf <name> <content>` | create file |
| `edit <name>` | overwrite file contents |
| `df <name>` | delete file |
| `mfd <src> <dst>` | move or rename |
| `crd <name>` | create directory |
| `cd <name>` | change directory |
| `peek <addr> [len]` | hex dump memory |
| `theme <name>` | switch theme |
| `font <name>` | switch font |
