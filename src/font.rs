// Drop any .psf file into assets/fonts/ — build.rs picks it up automatically.
include!(concat!(env!("OUT_DIR"), "/fonts_generated.rs"));

const PSF1_MAGIC: u16 = 0x0436;
const PSF2_MAGIC: u32 = 0x864ab572;

pub fn load(data: &[u8]) -> bool {
    if data.len() < 4 { return false; }

    // PSF1 header: [magic_lo, magic_hi, mode, charsize]
    if u16::from_le_bytes([data[0], data[1]]) == PSF1_MAGIC && data.len() >= 4 {
        let mode     = data[2];
        let charsize = data[3] as usize;
        let glyphs   = if mode & 0x01 != 0 { 512usize } else { 256 };
        let body     = &data[4..];
        if body.len() >= glyphs * charsize {
            load_raw(body, charsize, glyphs);
            return true;
        }
    }

    // PSF2 header (32 bytes):
    //   0-3   magic      8-11  headersize   16-19 length (glyph count)
    //   4-7   version    12-15 flags        20-23 charsize
    //   24-27 height     28-31 width
    if u32::from_le_bytes([data[0], data[1], data[2], data[3]]) == PSF2_MAGIC
        && data.len() >= 32
    {
        let hdrsize    = u32::from_le_bytes([data[8],  data[9],  data[10], data[11]]) as usize;
        let num_glyphs = u32::from_le_bytes([data[16], data[17], data[18], data[19]]) as usize;
        let charsize   = u32::from_le_bytes([data[20], data[21], data[22], data[23]]) as usize;
        let body       = &data[hdrsize..];
        if body.len() >= num_glyphs * charsize {
            load_raw(body, charsize, num_glyphs.min(512));
            return true;
        }
    }

    false
}

fn load_raw(data: &[u8], charsize: usize, count: usize) {
    use x86_64::instructions::port::Port;
    let mut seq_i: Port<u8> = Port::new(0x3C4);
    let mut seq_d: Port<u8> = Port::new(0x3C5);
    let mut gfx_i: Port<u8> = Port::new(0x3CE);
    let mut gfx_d: Port<u8> = Port::new(0x3CF);

    unsafe {
        // Switch to planar write mode so we can write font data to plane 2.
        // Plane 2 holds the character generator bitmaps at 0xA0000.
        seq_i.write(0x02); seq_d.write(0x04); // write mask  → plane 2 only
        seq_i.write(0x04); seq_d.write(0x07); // mem mode    → sequential
        gfx_i.write(0x04); gfx_d.write(0x02); // read map    → plane 2
        gfx_i.write(0x05); gfx_d.write(0x00); // gfx mode    → write mode 0
        gfx_i.write(0x06); gfx_d.write(0x04); // misc        → A000h–AFFFh

        let mem = 0xA0000usize as *mut u8;
        for g in 0..count {
            for row in 0..charsize {
                let byte = data.get(g * charsize + row).copied().unwrap_or(0);
                core::ptr::write_volatile(mem.add(g * 32 + row), byte);
            }
            // Each glyph slot is 32 bytes; zero the unused rows.
            for row in charsize..32 {
                core::ptr::write_volatile(mem.add(g * 32 + row), 0);
            }
        }

        // Restore text-mode VGA state (planes 0+1, B800h window).
        seq_i.write(0x02); seq_d.write(0x03); // write mask  → planes 0+1
        seq_i.write(0x04); seq_d.write(0x03); // mem mode    → normal
        gfx_i.write(0x04); gfx_d.write(0x00); // read map    → plane 0
        gfx_i.write(0x05); gfx_d.write(0x10); // gfx mode    → odd/even
        gfx_i.write(0x06); gfx_d.write(0x0E); // misc        → B800h–BFFFh
    }
}
