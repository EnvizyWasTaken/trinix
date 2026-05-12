use crate::ata;
use spin::Mutex;

const END_OF_CHAIN:   u32 = 0xFFFF_FFF8;
const ENTRY_FILE:     u8  = 0x85;
const ENTRY_STREAM:   u8  = 0xC0;
const ENTRY_NAME:     u8  = 0xC1;
const ENTRY_EOD:      u8  = 0x00;
const ENTRY_BITMAP:   u8  = 0x81;
const ATTR_DIRECTORY: u16 = 0x0010;
const ATTR_ARCHIVE:   u16 = 0x0020;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExFatError {
    NotExFat,
    Ata(ata::AtaError),
    FileNotFound,
    NotADirectory,
    SectorSizeUnsupported,
    NoSpace,
    NameTooLong,
}

impl From<ata::AtaError> for ExFatError {
    fn from(e: ata::AtaError) -> Self { ExFatError::Ata(e) }
}

struct Volume {
    fat_lba:             u32,
    cluster_heap_lba:    u32,
    root_cluster:        u32,
    sectors_per_cluster: u32,
    cluster_count:       u32,
    bitmap_cluster:      u32,
}

pub struct DirEntry {
    pub name:          [u8; 255],
    pub name_len:      usize,
    pub size:          u64,
    pub first_cluster: u32,
    pub is_dir:        bool,
}

impl DirEntry {
    pub const fn empty() -> Self {
        DirEntry { name: [0u8; 255], name_len: 0, size: 0, first_cluster: 0, is_dir: false }
    }
    pub fn name_bytes(&self) -> &[u8] { &self.name[..self.name_len] }
}

struct ParseState {
    active:        bool,
    is_dir:        bool,
    total_sec:     u8,
    seen_sec:      usize,
    name_len:      usize,
    name:          [u8; 255],
    size:          u64,
    first_cluster: u32,
}

impl ParseState {
    const fn new() -> Self {
        ParseState {
            active: false, is_dir: false, total_sec: 0, seen_sec: 0,
            name_len: 0, name: [0u8; 255], size: 0, first_cluster: 0,
        }
    }
    fn reset(&mut self) { *self = ParseState::new(); }
}

struct ExFatInner {
    vol:             Volume,
    present:         bool,
    current_cluster: u32,
}

impl ExFatInner {
    const fn new() -> Self {
        ExFatInner {
            vol: Volume {
                fat_lba: 0, cluster_heap_lba: 0, root_cluster: 0,
                sectors_per_cluster: 0, cluster_count: 0, bitmap_cluster: 0,
            },
            present:         false,
            current_cluster: 0,
        }
    }

    fn init(&mut self) -> Result<(), ExFatError> {
        let mut buf = [0u8; 512];
        ata::read(0, &mut buf)?;
        if &buf[3..11] != b"EXFAT   " { return Err(ExFatError::NotExFat); }
        if buf[108] != 9              { return Err(ExFatError::SectorSizeUnsupported); }

        self.vol = Volume {
            fat_lba:             u32_le(&buf, 80),
            cluster_heap_lba:    u32_le(&buf, 88),
            root_cluster:        u32_le(&buf, 96),
            sectors_per_cluster: 1u32 << buf[109],
            cluster_count:       u32_le(&buf, 92),
            bitmap_cluster:      0,
        };
        self.present         = true;
        self.current_cluster = self.vol.root_cluster;
        self.find_bitmap()?;
        Ok(())
    }

    fn find_bitmap(&mut self) -> Result<(), ExFatError> {
        let mut cluster = self.vol.root_cluster;
        loop {
            let base = self.cluster_to_lba(cluster);
            for s in 0..self.vol.sectors_per_cluster {
                let mut buf = [0u8; 512];
                ata::read(base + s, &mut buf)?;
                for i in 0..16usize {
                    let e = &buf[i * 32..(i + 1) * 32];
                    if e[0] == ENTRY_EOD    { return Ok(()); }
                    if e[0] == ENTRY_BITMAP {
                        self.vol.bitmap_cluster = u32_le(e, 20);
                        return Ok(());
                    }
                }
            }
            match self.fat_next(cluster)? {
                Some(next) => cluster = next,
                None       => return Ok(()),
            }
        }
    }

    fn cluster_to_lba(&self, cluster: u32) -> u32 {
        self.vol.cluster_heap_lba + (cluster - 2) * self.vol.sectors_per_cluster
    }

    fn fat_next(&self, cluster: u32) -> Result<Option<u32>, ExFatError> {
        let byte_off = cluster * 4;
        let mut buf  = [0u8; 512];
        ata::read(self.vol.fat_lba + byte_off / 512, &mut buf)?;
        let next = u32_le(&buf, (byte_off % 512) as usize);
        if next >= END_OF_CHAIN { Ok(None) } else { Ok(Some(next)) }
    }

    fn fat_write(&self, cluster: u32, value: u32) -> Result<(), ExFatError> {
        let byte_off = cluster * 4;
        let sector   = self.vol.fat_lba + byte_off / 512;
        let off      = (byte_off % 512) as usize;
        let mut buf  = [0u8; 512];
        ata::read(sector, &mut buf)?;
        buf[off..off + 4].copy_from_slice(&value.to_le_bytes());
        ata::write(sector, &buf)?;
        Ok(())
    }

    fn bitmap_set(&self, cluster: u32, used: bool) -> Result<(), ExFatError> {
        let idx      = cluster - 2;
        let byte_off = idx / 8;
        let bit      = (idx % 8) as u8;
        let sector   = self.cluster_to_lba(self.vol.bitmap_cluster) + byte_off / 512;
        let off      = (byte_off % 512) as usize;
        let mut buf  = [0u8; 512];
        ata::read(sector, &mut buf)?;
        if used { buf[off] |= 1 << bit; } else { buf[off] &= !(1 << bit); }
        ata::write(sector, &buf)?;
        Ok(())
    }

    fn find_free_cluster(&self) -> Result<u32, ExFatError> {
        let base           = self.cluster_to_lba(self.vol.bitmap_cluster);
        let bitmap_sectors = ((self.vol.cluster_count as usize + 7) / 8 + 511) / 512;
        for s in 0..bitmap_sectors {
            let mut buf = [0u8; 512];
            ata::read(base + s as u32, &mut buf)?;
            for b in 0..512usize {
                if buf[b] == 0xFF { continue; }
                for bit in 0..8u32 {
                    if buf[b] & (1 << bit) == 0 {
                        let cluster = 2 + (s * 512 + b) as u32 * 8 + bit;
                        if cluster < self.vol.cluster_count + 2 {
                            return Ok(cluster);
                        }
                    }
                }
            }
        }
        Err(ExFatError::NoSpace)
    }

    fn alloc_cluster(&self) -> Result<u32, ExFatError> {
        let c = self.find_free_cluster()?;
        self.bitmap_set(c, true)?;
        self.fat_write(c, END_OF_CHAIN)?;
        Ok(c)
    }

    fn free_chain(&self, mut cluster: u32) -> Result<(), ExFatError> {
        loop {
            let next = self.fat_next(cluster)?;
            self.bitmap_set(cluster, false)?;
            self.fat_write(cluster, 0)?;
            match next {
                Some(c) => cluster = c,
                None    => return Ok(()),
            }
        }
    }

    fn process_entry(e: &[u8], state: &mut ParseState) -> Option<DirEntry> {
        match e[0] {
            ENTRY_EOD => {}
            ENTRY_FILE => {
                state.reset();
                state.active    = true;
                state.is_dir    = u16_le(e, 4) & ATTR_DIRECTORY != 0;
                state.total_sec = e[1];
            }
            ENTRY_STREAM if state.active => {
                state.name_len      = e[3] as usize;
                state.size          = u64_le(e, 24);
                state.first_cluster = u32_le(e, 20);
                state.seen_sec     += 1;
            }
            ENTRY_NAME if state.active => {
                let base = (state.seen_sec - 1) * 15;
                for j in 0..15usize {
                    if base + j >= state.name_len { break; }
                    let lo = e[2 + j * 2];
                    let hi = e[3 + j * 2];
                    state.name[base + j] = if hi == 0 { lo } else { b'?' };
                }
                state.seen_sec += 1;
                if state.seen_sec >= state.total_sec as usize {
                    let de = DirEntry {
                        name:          state.name,
                        name_len:      state.name_len,
                        size:          state.size,
                        first_cluster: state.first_cluster,
                        is_dir:        state.is_dir,
                    };
                    state.reset();
                    return Some(de);
                }
            }
            b if b & 0x80 == 0 => { state.reset(); }
            _ => {}
        }
        None
    }

    fn is_eod(e: &[u8]) -> bool { e[0] == ENTRY_EOD }

    fn walk_dir(&self, dir_cluster: u32, out: &mut [DirEntry]) -> Result<usize, ExFatError> {
        let mut count   = 0usize;
        let mut state   = ParseState::new();
        let mut cluster = dir_cluster;
        'outer: loop {
            let base = self.cluster_to_lba(cluster);
            for s in 0..self.vol.sectors_per_cluster {
                let mut buf = [0u8; 512];
                ata::read(base + s, &mut buf)?;
                for i in 0..16usize {
                    let entry = &buf[i * 32..(i + 1) * 32];
                    if Self::is_eod(entry) { break 'outer; }
                    if let Some(de) = Self::process_entry(entry, &mut state) {
                        if count < out.len() { out[count] = de; count += 1; }
                        if count >= out.len() { break 'outer; }
                    }
                }
            }
            match self.fat_next(cluster)? {
                Some(next) => cluster = next,
                None       => break,
            }
        }
        Ok(count)
    }

    fn find_in_dir(&self, dir_cluster: u32, name: &[u8]) -> Result<DirEntry, ExFatError> {
        let mut state   = ParseState::new();
        let mut cluster = dir_cluster;
        loop {
            let base = self.cluster_to_lba(cluster);
            for s in 0..self.vol.sectors_per_cluster {
                let mut buf = [0u8; 512];
                ata::read(base + s, &mut buf)?;
                for i in 0..16usize {
                    let entry = &buf[i * 32..(i + 1) * 32];
                    if Self::is_eod(entry) { return Err(ExFatError::FileNotFound); }
                    if let Some(de) = Self::process_entry(entry, &mut state) {
                        if de.name_bytes().eq_ignore_ascii_case(name) {
                            return Ok(de);
                        }
                    }
                }
            }
            match self.fat_next(cluster)? {
                Some(next) => cluster = next,
                None       => return Err(ExFatError::FileNotFound),
            }
        }
    }

    fn find_entry_set_index(&self, dir_cluster: u32, name: &[u8]) -> Result<(usize, usize), ExFatError> {
        let mut state     = ParseState::new();
        let mut cluster   = dir_cluster;
        let mut abs_idx   = 0usize;
        let mut set_start = 0usize;
        let mut set_count = 0usize;
        loop {
            let base = self.cluster_to_lba(cluster);
            for s in 0..self.vol.sectors_per_cluster {
                let mut buf = [0u8; 512];
                ata::read(base + s, &mut buf)?;
                for i in 0..16usize {
                    let e = &buf[i * 32..(i + 1) * 32];
                    if e[0] == ENTRY_EOD  { return Err(ExFatError::FileNotFound); }
                    if e[0] == ENTRY_FILE { set_start = abs_idx; set_count = 1 + e[1] as usize; }
                    if let Some(de) = Self::process_entry(e, &mut state) {
                        if de.name_bytes().eq_ignore_ascii_case(name) {
                            return Ok((set_start, set_count));
                        }
                    }
                    abs_idx += 1;
                }
            }
            match self.fat_next(cluster)? {
                Some(next) => cluster = next,
                None       => return Err(ExFatError::FileNotFound),
            }
        }
    }

    fn write_entry_at(&self, dir_cluster: u32, abs_idx: usize, entry: &[u8; 32]) -> Result<(), ExFatError> {
        let epc     = 16 * self.vol.sectors_per_cluster as usize;
        let mut cl  = dir_cluster;
        let mut rem = abs_idx;
        loop {
            if rem < epc {
                let lba     = self.cluster_to_lba(cl) + (rem / 16) as u32;
                let off     = (rem % 16) * 32;
                let mut buf = [0u8; 512];
                ata::read(lba, &mut buf)?;
                buf[off..off + 32].copy_from_slice(entry);
                ata::write(lba, &buf)?;
                return Ok(());
            }
            rem -= epc;
            cl = match self.fat_next(cl)? {
                Some(next) => next,
                None => {
                    let new_c = self.alloc_cluster()?;
                    self.fat_write(cl, new_c)?;
                    let empty = [0u8; 512];
                    for s in 0..self.vol.sectors_per_cluster {
                        ata::write(self.cluster_to_lba(new_c) + s, &empty)?;
                    }
                    new_c
                }
            };
        }
    }

    fn mark_entry_deleted(&self, dir_cluster: u32, abs_idx: usize) -> Result<(), ExFatError> {
        let epc     = 16 * self.vol.sectors_per_cluster as usize;
        let mut cl  = dir_cluster;
        let mut rem = abs_idx;
        loop {
            if rem < epc {
                let lba     = self.cluster_to_lba(cl) + (rem / 16) as u32;
                let off     = (rem % 16) * 32;
                let mut buf = [0u8; 512];
                ata::read(lba, &mut buf)?;
                buf[off] &= !0x80;
                ata::write(lba, &buf)?;
                return Ok(());
            }
            rem -= epc;
            cl = match self.fat_next(cl)? {
                Some(next) => next,
                None       => return Err(ExFatError::FileNotFound),
            };
        }
    }

    fn find_eod_index(&self, dir_cluster: u32) -> Result<usize, ExFatError> {
        let mut idx     = 0usize;
        let mut cluster = dir_cluster;
        loop {
            let base = self.cluster_to_lba(cluster);
            for s in 0..self.vol.sectors_per_cluster {
                let mut buf = [0u8; 512];
                ata::read(base + s, &mut buf)?;
                for i in 0..16usize {
                    if buf[i * 32] == ENTRY_EOD { return Ok(idx); }
                    idx += 1;
                }
            }
            cluster = match self.fat_next(cluster)? {
                Some(next) => next,
                None => {
                    let new_c = self.alloc_cluster()?;
                    self.fat_write(cluster, new_c)?;
                    let empty = [0u8; 512];
                    for s in 0..self.vol.sectors_per_cluster {
                        ata::write(self.cluster_to_lba(new_c) + s, &empty)?;
                    }
                    new_c
                }
            };
        }
    }

    fn build_entry_set(
        &self,
        name:          &[u8],
        first_cluster: u32,
        size:          u64,
        attr:          u16,
        file_out:      &mut [u8; 32],
        stream_out:    &mut [u8; 32],
        name_bufs:     &mut [[u8; 32]; 17],
    ) -> usize {
        let name_len     = name.len().min(255);
        let name_entries = (name_len + 14) / 15;
        let sec_count    = (1 + name_entries) as u8;

        file_out[0] = ENTRY_FILE;
        file_out[1] = sec_count;
        file_out[4] = (attr & 0xFF) as u8;
        file_out[5] = (attr >> 8)   as u8;

        stream_out[0] = ENTRY_STREAM;
        stream_out[1] = 0x01;
        stream_out[3] = name_len as u8;
        let nh = name_hash(name);
        stream_out[4]  = nh as u8;
        stream_out[5]  = (nh >> 8) as u8;
        stream_out[8..16].copy_from_slice(&size.to_le_bytes());
        stream_out[20..24].copy_from_slice(&first_cluster.to_le_bytes());
        stream_out[24..32].copy_from_slice(&size.to_le_bytes());

        for ni in 0..name_entries {
            name_bufs[ni][0] = ENTRY_NAME;
            name_bufs[ni][1] = 0x01;
            let base = ni * 15;
            for j in 0..15usize {
                let idx = base + j;
                if idx >= name_len { break; }
                let ch    = name[idx];
                let upper = if ch >= b'a' && ch <= b'z' { ch - 32 } else { ch };
                name_bufs[ni][2 + j * 2] = upper;
                name_bufs[ni][3 + j * 2] = 0;
            }
        }

        let checksum = {
            let mut sum: u16 = 0;
            let entries = core::iter::once(&*file_out)
                .chain(core::iter::once(&*stream_out))
                .chain(name_bufs[..name_entries].iter());
            for (ei, entry) in entries.enumerate() {
                for (bi, &b) in entry.iter().enumerate() {
                    if ei == 0 && (bi == 2 || bi == 3) { continue; }
                    sum = sum.rotate_right(1).wrapping_add(b as u16);
                }
            }
            sum
        };
        file_out[2] = checksum as u8;
        file_out[3] = (checksum >> 8) as u8;

        name_entries
    }

    fn append_dir_entry(&self, dir_cluster: u32, name: &[u8], first_cluster: u32, size: u64) -> Result<(), ExFatError> {
        let mut file_entry   = [0u8; 32];
        let mut stream_entry = [0u8; 32];
        let mut name_bufs    = [[0u8; 32]; 17];

        let name_entries = self.build_entry_set(
            name, first_cluster, size, ATTR_ARCHIVE,
            &mut file_entry, &mut stream_entry, &mut name_bufs,
        );

        let eod = self.find_eod_index(dir_cluster)?;
        self.write_entry_at(dir_cluster, eod,     &file_entry)?;
        self.write_entry_at(dir_cluster, eod + 1, &stream_entry)?;
        for ni in 0..name_entries {
            self.write_entry_at(dir_cluster, eod + 2 + ni, &name_bufs[ni])?;
        }
        Ok(())
    }

    pub fn ls(&self, out: &mut [DirEntry]) -> Result<usize, ExFatError> {
        if !self.present { return Err(ExFatError::NotExFat); }
        self.walk_dir(self.current_cluster, out)
    }

    pub fn find(&self, name: &[u8]) -> Result<DirEntry, ExFatError> {
        if !self.present { return Err(ExFatError::NotExFat); }
        self.find_in_dir(self.current_cluster, name)
    }

    pub fn chdir(&mut self, name: &[u8]) -> Result<(), ExFatError> {
        if !self.present { return Err(ExFatError::NotExFat); }
        if name == b"/" || name == b".." {
            self.current_cluster = self.vol.root_cluster;
            return Ok(());
        }
        let entry = self.find_in_dir(self.current_cluster, name)?;
        if !entry.is_dir { return Err(ExFatError::NotADirectory); }
        self.current_cluster = entry.first_cluster;
        Ok(())
    }

    pub fn read(&self, name: &[u8], buf: &mut [u8]) -> Result<u64, ExFatError> {
        if !self.present { return Err(ExFatError::NotExFat); }
        let entry   = self.find_in_dir(self.current_cluster, name)?;
        let to_read = (entry.size as usize).min(buf.len());
        let mut written = 0usize;
        let mut cluster = entry.first_cluster;
        'outer: loop {
            let base = self.cluster_to_lba(cluster);
            for s in 0..self.vol.sectors_per_cluster {
                if written >= to_read { break 'outer; }
                let mut sector_buf = [0u8; 512];
                ata::read(base + s, &mut sector_buf)?;
                let chunk = (to_read - written).min(512);
                buf[written..written + chunk].copy_from_slice(&sector_buf[..chunk]);
                written += chunk;
            }
            match self.fat_next(cluster)? {
                Some(next) => cluster = next,
                None       => break,
            }
        }
        Ok(written as u64)
    }

    pub fn create(&self, name: &[u8], data: &[u8]) -> Result<(), ExFatError> {
        if !self.present               { return Err(ExFatError::NotExFat); }
        if name.len() > 255            { return Err(ExFatError::NameTooLong); }
        if self.vol.bitmap_cluster == 0 { return Err(ExFatError::NoSpace); }

        let bytes_per_cluster = self.vol.sectors_per_cluster as usize * 512;
        let clusters_needed   = ((data.len() + bytes_per_cluster - 1) / bytes_per_cluster).max(1);

        let first_cluster = self.alloc_cluster()?;
        let mut prev = first_cluster;
        for _ in 1..clusters_needed {
            let next = self.alloc_cluster()?;
            self.fat_write(prev, next)?;
            prev = next;
        }

        let mut cluster         = first_cluster;
        let mut offset          = 0usize;
        let mut sector_in_clust = 0u32;
        while offset < data.len() {
            let mut sector_buf = [0u8; 512];
            let end = (offset + 512).min(data.len());
            sector_buf[..end - offset].copy_from_slice(&data[offset..end]);
            ata::write(self.cluster_to_lba(cluster) + sector_in_clust, &sector_buf)?;
            offset          += end - offset;
            sector_in_clust += 1;
            if sector_in_clust >= self.vol.sectors_per_cluster {
                sector_in_clust = 0;
                if let Some(next) = self.fat_next(cluster)? { cluster = next; }
            }
        }

        self.append_dir_entry(self.current_cluster, name, first_cluster, data.len() as u64)
    }

    pub fn create_dir(&self, name: &[u8]) -> Result<(), ExFatError> {
        if !self.present               { return Err(ExFatError::NotExFat); }
        if name.len() > 255            { return Err(ExFatError::NameTooLong); }
        if self.vol.bitmap_cluster == 0 { return Err(ExFatError::NoSpace); }

        let cluster = self.alloc_cluster()?;
        let empty   = [0u8; 512];
        for s in 0..self.vol.sectors_per_cluster {
            ata::write(self.cluster_to_lba(cluster) + s, &empty)?;
        }

        let mut file_entry   = [0u8; 32];
        let mut stream_entry = [0u8; 32];
        let mut name_bufs    = [[0u8; 32]; 17];

        let name_entries = self.build_entry_set(
            name, cluster, 0, ATTR_DIRECTORY,
            &mut file_entry, &mut stream_entry, &mut name_bufs,
        );

        let eod = self.find_eod_index(self.current_cluster)?;
        self.write_entry_at(self.current_cluster, eod,     &file_entry)?;
        self.write_entry_at(self.current_cluster, eod + 1, &stream_entry)?;
        for ni in 0..name_entries {
            self.write_entry_at(self.current_cluster, eod + 2 + ni, &name_bufs[ni])?;
        }
        Ok(())
    }

    pub fn delete(&self, name: &[u8]) -> Result<(), ExFatError> {
        if !self.present { return Err(ExFatError::NotExFat); }
        let entry                  = self.find_in_dir(self.current_cluster, name)?;
        let (set_start, set_count) = self.find_entry_set_index(self.current_cluster, name)?;
        self.free_chain(entry.first_cluster)?;
        for i in 0..set_count {
            self.mark_entry_deleted(self.current_cluster, set_start + i)?;
        }
        Ok(())
    }

    pub fn move_entry(&self, src: &[u8], dst: &[u8]) -> Result<(), ExFatError> {
        if !self.present { return Err(ExFatError::NotExFat); }
        let entry                  = self.find_in_dir(self.current_cluster, src)?;
        let (set_start, set_count) = self.find_entry_set_index(self.current_cluster, src)?;

        let (dst_cluster, new_name) = match self.find_in_dir(self.current_cluster, dst) {
            Ok(de) if de.is_dir           => (de.first_cluster, src),
            Ok(_)                         => return Err(ExFatError::NotADirectory),
            Err(ExFatError::FileNotFound) => (self.current_cluster, dst),
            Err(e)                        => return Err(e),
        };

        self.append_dir_entry(dst_cluster, new_name, entry.first_cluster, entry.size)?;
        for i in 0..set_count {
            self.mark_entry_deleted(self.current_cluster, set_start + i)?;
        }
        Ok(())
    }
}

fn name_hash(name: &[u8]) -> u16 {
    let mut hash: u16 = 0;
    for &b in name {
        let upper = if b >= b'a' && b <= b'z' { b - 32 } else { b };
        hash = hash.rotate_right(1).wrapping_add(upper as u16);
        hash = hash.rotate_right(1);
    }
    hash
}

#[inline] fn u16_le(b: &[u8], o: usize) -> u16 { u16::from_le_bytes([b[o], b[o+1]]) }
#[inline] fn u32_le(b: &[u8], o: usize) -> u32 { u32::from_le_bytes([b[o], b[o+1], b[o+2], b[o+3]]) }
#[inline] fn u64_le(b: &[u8], o: usize) -> u64 { u64::from_le_bytes([b[o], b[o+1], b[o+2], b[o+3], b[o+4], b[o+5], b[o+6], b[o+7]]) }

static FS: Mutex<ExFatInner> = Mutex::new(ExFatInner::new());

pub fn init()                            -> Result<(), ExFatError>       { FS.lock().init() }
pub fn ls(out: &mut [DirEntry])          -> Result<usize, ExFatError>    { FS.lock().ls(out) }
pub fn find(name: &[u8])                 -> Result<DirEntry, ExFatError> { FS.lock().find(name) }
pub fn read(name: &[u8], buf: &mut [u8]) -> Result<u64, ExFatError>      { FS.lock().read(name, buf) }
pub fn create(name: &[u8], data: &[u8])  -> Result<(), ExFatError>       { FS.lock().create(name, data) }
pub fn create_dir(name: &[u8])           -> Result<(), ExFatError>       { FS.lock().create_dir(name) }
pub fn delete(name: &[u8])               -> Result<(), ExFatError>       { FS.lock().delete(name) }
pub fn move_file(src: &[u8], dst: &[u8]) -> Result<(), ExFatError>       { FS.lock().move_entry(src, dst) }
pub fn chdir(name: &[u8])                -> Result<(), ExFatError>       { FS.lock().chdir(name) }
