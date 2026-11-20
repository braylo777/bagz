//! Write the embedded web app to a folder: `cargo run -p bagz-web --bin export -- dist`.

use std::{env, fs, path::PathBuf};

fn main() -> std::io::Result<()> {
    let out = PathBuf::from(env::args().nth(1).unwrap_or_else(|| "dist".into()));
    let mut n = 0;
    for a in bagz_web::assets() {
        let dest = out.join(a.path.trim_start_matches('/'));
        if let Some(dir) = dest.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(&dest, a.body)?;
        n += 1;
    }
    println!("wrote {n} files to {}", out.display());
    Ok(())
}
