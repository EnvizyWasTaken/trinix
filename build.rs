use std::{env, fs, path::Path};

fn main() {
    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap();
    let fonts_dir = Path::new(&manifest).join("assets").join("fonts");
    let out_dir   = env::var("OUT_DIR").unwrap();

    // Re-run whenever the fonts directory changes
    println!("cargo:rerun-if-changed=assets/fonts/");

    let mut entries: Vec<(String, String)> = Vec::new();

    if let Ok(rd) = fs::read_dir(&fonts_dir) {
        let mut paths: Vec<_> = rd
            .flatten()
            .filter(|e| e.path().extension().map_or(false, |x| x == "psf"))
            .collect();
        paths.sort_by_key(|e| e.file_name());

        for entry in paths {
            let path     = entry.path();
            let name     = path.file_stem().unwrap().to_str().unwrap().to_string();
            let abs_path = path.to_str().unwrap().to_string();
            println!("cargo:rerun-if-changed={}", abs_path);
            entries.push((name, abs_path));
        }
    }

    // Generate:  pub static FONTS: &[(&str, &[u8])] = &[ ... ];
    let mut code = String::from("pub static FONTS: &[(&str, &[u8])] = &[\n");
    for (name, path) in &entries {
        code.push_str(&format!("    ({name:?}, include_bytes!({path:?})),\n"));
    }
    code.push_str("];\n");

    fs::write(Path::new(&out_dir).join("fonts_generated.rs"), code).unwrap();
}
