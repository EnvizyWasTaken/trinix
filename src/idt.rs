use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

pub static IDT: spin::Lazy<InterruptDescriptorTable> = spin::Lazy::new(|| {
    let mut idt = InterruptDescriptorTable::new();
    idt.double_fault.set_handler_fn(double_fault_handler);
    idt[32].set_handler_fn(timer_interrupt_handler);
    idt[33].set_handler_fn(keyboard_interrupt_handler);
    idt
});

pub fn init() {
    IDT.load();
    // Remap PIC: IRQ0-7 → vectors 32-39, IRQ8-15 → vectors 40-47
    unsafe {
        // Master PIC
        Port::<u8>::new(0x20).write(0x11); // init
        Port::<u8>::new(0x21).write(0x20); // offset 32
        Port::<u8>::new(0x21).write(0x04); // slave at IRQ2
        Port::<u8>::new(0x21).write(0x01); // 8086 mode
        Port::<u8>::new(0x21).write(0xFD); // mask all except IRQ1 (keyboard)
        // Slave PIC
        Port::<u8>::new(0xA0).write(0x11);
        Port::<u8>::new(0xA1).write(0x28); // offset 40
        Port::<u8>::new(0xA1).write(0x02);
        Port::<u8>::new(0xA1).write(0x01);
        Port::<u8>::new(0xA1).write(0xFF); // mask all
    }
    x86_64::instructions::interrupts::enable();
}
