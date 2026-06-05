pub fn load() {
    load_hostname();
    load_conf();
}

fn load_hostname() {
    let mut buf = [0u8; 64];
    let Ok(n) = crate::exfat::read_path(b"etc/hostname", &mut buf) else { return };
    let name = trim(&buf[..n as usize]);
    if !name.is_empty() {
        crate::env::ENV.lock().set(b"HOSTNAME", name);
    }
}

fn load_conf() {
    let mut buf = [0u8; 4096];
    let n = match crate::exfat::read_path(b"etc/trinix.conf", &mut buf) {
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
        _           => { crate::env::ENV.lock().set(key, val); }
    }
}

pub fn log_boot(msg: &[u8]) {
    let mut buf = [0u8; 4096];
    let existing = crate::exfat::read_path(b"var/log/boot.log", &mut buf)
        .unwrap_or(0) as usize;

    let msg_len = msg.len().min(4096 - existing - 1);
    if existing + msg_len + 1 >= 4096 { return; }

    buf[existing..existing + msg_len].copy_from_slice(&msg[..msg_len]);
    buf[existing + msg_len] = b'\n';

    let _ = crate::exfat::delete_path(b"var/log/boot.log");
    let _ = crate::exfat::create_path(b"var/log/boot.log", &buf[..existing + msg_len + 1]);
}

fn trim(b: &[u8]) -> &[u8] {
    let s = b.iter().position(|&c| c != b' ' && c != b'\t' && c != b'\r')
        .unwrap_or(b.len());
    let e = b.iter().rposition(|&c| c != b' ' && c != b'\t' && c != b'\r')
        .map(|i| i + 1)
        .unwrap_or(0);
    if s >= e { &[] } else { &b[s..e] }
}
