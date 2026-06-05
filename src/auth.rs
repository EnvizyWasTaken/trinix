pub fn authenticate(username: &[u8], password: &[u8]) -> bool {
    let mut buf = [0u8; 512];
    let n = match crate::exfat::read_path(b"etc/passwd", &mut buf) {
        Ok(n)  => n as usize,
        Err(_) => return false,
    };

    let data = &buf[..n];
    let mut pos = 0usize;

    while pos < data.len() {
        let end = data[pos..].iter().position(|&b| b == b'\n')
            .map(|i| pos + i).unwrap_or(data.len());
        let line = &data[pos..end];
        pos = end + 1;
        if line.is_empty() { continue; }
        let Some(colon) = line.iter().position(|&b| b == b':') else { continue };
        if &line[..colon] == username && &line[colon + 1..] == password {
            return true;
        }
    }
    false
}

pub fn user_exists(username: &[u8]) -> bool {
    let mut buf = [0u8; 512];
    let n = match crate::exfat::read_path(b"etc/passwd", &mut buf) {
        Ok(n)  => n as usize,
        Err(_) => return false,
    };

    let data = &buf[..n];
    let mut pos = 0usize;

    while pos < data.len() {
        let end = data[pos..].iter().position(|&b| b == b'\n')
            .map(|i| pos + i).unwrap_or(data.len());
        let line = &data[pos..end];
        pos = end + 1;
        if line.is_empty() { continue; }
        let Some(colon) = line.iter().position(|&b| b == b':') else { continue };
        if &line[..colon] == username { return true; }
    }
    false
}
