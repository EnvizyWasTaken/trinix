pub fn init(hz: u32) {
    let divisor = 1_193_182u32 / hz;
    unsafe {
        Port::<u8>::new(0x43).write(0x36);
        Port::<u8>::new(0x40).write((divisor & 0xFF) as u8);
        Port::<u8>::new(0x40).write((divisor >> 8) as u8);
        let mask = Port::<u8>::new(0x21).read();
        Port::<u8>::new(0x21).write(mask & 0xFE);
    }
}
