use x86_64::instructions::port::Port;

const PORT_DATA:         u16 = 0x1F0;
const PORT_ERROR:        u16 = 0x1F1;
const PORT_SECTOR_COUNT: u16 = 0x1F2;
const PORT_LBA_LO:       u16 = 0x1F3;
const PORT_LBA_MID:      u16 = 0x1F4;
const PORT_LBA_HI:       u16 = 0x1F5;
const PORT_DRIVE:        u16 = 0x1F6;
const PORT_STATUS:       u16 = 0x1F7;
const PORT_COMMAND:      u16 = 0x1F7;
const PORT_ALT_STATUS:   u16 = 0x3F6;

const CMD_READ_SECTORS:  u8  = 0x20;
const CMD_WRITE_SECTORS: u8  = 0x30;
const CMD_IDENTIFY:      u8  = 0xEC;
const CMD_FLUSH:         u8  = 0xE7;

const STATUS_ERR:        u8  = 0x01;
const STATUS_DRQ:        u8  = 0x08;
const STATUS_BSY:        u8  = 0x80;
const STATUS_DRDY:       u8  = 0x40;

const DRIVE_MASTER:      u8  = 0xF0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AtaError {
    Timeout,
    DriveFault,
    NoDevice,
    ReadError,
    WriteError,
}

pub struct AtaDrive {
    present:      bool,
    sector_count: u32,
}

impl AtaDrive {
    pub const fn new() -> Self {
        AtaDrive { present: false, sector_count: 0 }
    }

    pub fn init(&mut self) -> Result<(), AtaError> {
        unsafe {
            Port::<u8>::new(PORT_DRIVE).write(DRIVE_MASTER);
            self.io_delay();
            self.io_delay();
            self.io_delay();
            self.io_delay();

            Port::<u8>::new(PORT_SECTOR_COUNT).write(0);
            Port::<u8>::new(PORT_LBA_LO).write(0);
            Port::<u8>::new(PORT_LBA_MID).write(0);
            Port::<u8>::new(PORT_LBA_HI).write(0);
            Port::<u8>::new(PORT_COMMAND).write(CMD_IDENTIFY);

            let status = Port::<u8>::new(PORT_STATUS).read();
            if status == 0 {
                return Err(AtaError::NoDevice);
            }

            self.wait_bsy()?;

            let mid = Port::<u8>::new(PORT_LBA_MID).read();
            let hi  = Port::<u8>::new(PORT_LBA_HI).read();
            if mid != 0 || hi != 0 {
                return Err(AtaError::NoDevice);
            }

            self.wait_drq()?;

            let mut identify = [0u16; 256];
            for word in identify.iter_mut() {
                *word = Port::<u16>::new(PORT_DATA).read();
            }

            self.sector_count = (identify[61] as u32) << 16 | (identify[60] as u32);
            self.present      = true;
        }
        Ok(())
    }

    pub fn read(&self, lba: u32, buf: &mut [u8; 512]) -> Result<(), AtaError> {
        if !self.present            { return Err(AtaError::NoDevice); }
        if lba >= self.sector_count { return Err(AtaError::ReadError); }

        unsafe {
            self.wait_bsy()?;
            Port::<u8>::new(PORT_DRIVE).write(DRIVE_MASTER | ((lba >> 24) & 0x0F) as u8);
            self.io_delay();
            self.io_delay();
            self.io_delay();
            self.io_delay();
            self.wait_ready()?;

            Port::<u8>::new(PORT_ERROR).write(0);
            Port::<u8>::new(PORT_SECTOR_COUNT).write(1);
            Port::<u8>::new(PORT_LBA_LO).write((lba & 0xFF)         as u8);
            Port::<u8>::new(PORT_LBA_MID).write(((lba >> 8)  & 0xFF) as u8);
            Port::<u8>::new(PORT_LBA_HI).write(((lba >> 16) & 0xFF)  as u8);
            Port::<u8>::new(PORT_COMMAND).write(CMD_READ_SECTORS);

            self.wait_drq()?;

            let status = Port::<u8>::new(PORT_STATUS).read();
            if status & STATUS_ERR != 0 { return Err(AtaError::ReadError); }

            for chunk in buf.chunks_exact_mut(2) {
                let word = Port::<u16>::new(PORT_DATA).read();
                chunk[0] = (word & 0xFF) as u8;
                chunk[1] = (word >> 8)   as u8;
            }
        }
        Ok(())
    }

    pub fn write(&self, lba: u32, buf: &[u8; 512]) -> Result<(), AtaError> {
        if !self.present            { return Err(AtaError::NoDevice); }
        if lba >= self.sector_count { return Err(AtaError::WriteError); }

        unsafe {
            self.wait_bsy()?;
            Port::<u8>::new(PORT_DRIVE).write(DRIVE_MASTER | ((lba >> 24) & 0x0F) as u8);
            self.io_delay();
            self.io_delay();
            self.io_delay();
            self.io_delay();
            self.wait_ready()?;

            Port::<u8>::new(PORT_ERROR).write(0);
            Port::<u8>::new(PORT_SECTOR_COUNT).write(1);
            Port::<u8>::new(PORT_LBA_LO).write((lba & 0xFF)         as u8);
            Port::<u8>::new(PORT_LBA_MID).write(((lba >> 8)  & 0xFF) as u8);
            Port::<u8>::new(PORT_LBA_HI).write(((lba >> 16) & 0xFF)  as u8);
            Port::<u8>::new(PORT_COMMAND).write(CMD_WRITE_SECTORS);

            self.wait_drq()?;

            let status = Port::<u8>::new(PORT_STATUS).read();
            if status & STATUS_ERR != 0 { return Err(AtaError::WriteError); }

            for chunk in buf.chunks_exact(2) {
                let word = (chunk[1] as u16) << 8 | (chunk[0] as u16);
                Port::<u16>::new(PORT_DATA).write(word);
            }

            Port::<u8>::new(PORT_COMMAND).write(CMD_FLUSH);
            self.wait_bsy()?;
        }
        Ok(())
    }

    pub fn sector_count(&self) -> u32 { self.sector_count }
    pub fn present(&self)      -> bool { self.present }

    unsafe fn wait_bsy(&self) -> Result<(), AtaError> {
        for _ in 0..100_000usize {
            let s = Port::<u8>::new(PORT_ALT_STATUS).read();
            if s & STATUS_BSY == 0 { return Ok(()); }
        }
        Err(AtaError::Timeout)
    }

    unsafe fn wait_ready(&self) -> Result<(), AtaError> {
        for _ in 0..100_000usize {
            let s = Port::<u8>::new(PORT_ALT_STATUS).read();
            if s & STATUS_BSY  != 0 { continue; }
            if s & STATUS_DRDY != 0 { return Ok(()); }
        }
        Err(AtaError::Timeout)
    }

    unsafe fn wait_drq(&self) -> Result<(), AtaError> {
        for _ in 0..100_000usize {
            let s = Port::<u8>::new(PORT_ALT_STATUS).read();
            if s & STATUS_ERR != 0 { return Err(AtaError::ReadError); }
            if s & STATUS_DRQ != 0 { return Ok(()); }
        }
        Err(AtaError::Timeout)
    }

    unsafe fn io_delay(&self) {
        for _ in 0..4 {
            Port::<u8>::new(PORT_ALT_STATUS).read();
        }
    }
}

pub static DRIVE: spin::Mutex<AtaDrive> = spin::Mutex::new(AtaDrive::new());

pub fn init() -> Result<(), AtaError> {
    DRIVE.lock().init()
}

pub fn read(lba: u32, buf: &mut [u8; 512]) -> Result<(), AtaError> {
    DRIVE.lock().read(lba, buf)
}

pub fn write(lba: u32, buf: &[u8; 512]) -> Result<(), AtaError> {
    DRIVE.lock().write(lba, buf)
}
