pub fn load() {
    let mut buf = [0u8; 4096];
    let n = match crate::exfat::read(b"etc/trinix.conf", &mut buf) {
        Ok(n)  => n as usize,
        Err(_) => return,
    };

    let data = &buf[..n];
    let mut line_start = 0usize;

    while line_start < data.len() {
        let line_end = data[line_start..]
            .iter()
            .position(|&b| b == b'\n')
            .map(|i| line_start + i)
            .unwrap_or(data.len());

        let line = trim(&data[line_start..line_end]);
        line_start = line_end + 1;

        if line.is_empty() || line[0] == b'#' { continue; }

        let Some(eq) = line.iter().position(|&b| b == b'=') else { continue };
        let key = trim(&line[..eq]);
        let val = trim(&line[eq + 1..]);
        if key.is_empty() || val.is_empty() { continue; }

        apply(key, val);
    }
}

fn apply(key: &[u8], val: &[u8]) {
    match key {
        b"theme"    => { crate::commands::cmd_theme(val); }
        b"font"     => { crate::commands::cmd_font(val); }
        b"hostname" => { crate::env::ENV.lock().set(b"HOSTNAME", val); }
        b"motd"     => {}
        _           => { crate::env::ENV.lock().set(key, val); }
    }
}

fn trim(b: &[u8]) -> &[u8] {
    let s = b.iter().position(|&c| c != b' ' && c != b'\t' && c != b'\r')
        .unwrap_or(b.len());
    let e = b.iter().rposition(|&c| c != b' ' && c != b'\t' && c != b'\r')
        .map(|i| i + 1)
        .unwrap_or(0);
    if s >= e { &[] } else { &b[s..e] }
}
