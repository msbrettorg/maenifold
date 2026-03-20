use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=MAENIFOLD_WEB_DIST");

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    let web_dist = if let Ok(path) = std::env::var("MAENIFOLD_WEB_DIST") {
        PathBuf::from(path)
    } else {
        manifest_dir.join("web-dist")
    };

    println!("cargo:rustc-env=MAENIFOLD_WEB_DIST={}", web_dist.display());
    emit_rerun_if_changed(&web_dist);
}

fn emit_rerun_if_changed(path: &Path) {
    println!("cargo:rerun-if-changed={}", path.display());
    if let Ok(read_dir) = std::fs::read_dir(path) {
        for entry in read_dir.flatten() {
            let p = entry.path();
            if p.is_dir() {
                emit_rerun_if_changed(&p);
            } else {
                println!("cargo:rerun-if-changed={}", p.display());
            }
        }
    }
}
