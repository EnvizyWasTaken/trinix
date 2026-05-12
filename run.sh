#!/bin/bash
cargo bootimage && \
qemu-system-x86_64 \
  -drive format=raw,file=target/x86_64-trinix/debug/bootimage-trinix-os.bin \
  -drive id=disk,if=none,format=raw,file=disk.img \
  -device ide-hd,drive=disk,bus=ide.1
