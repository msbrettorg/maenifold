fn main() {
    println!("cargo:rerun-if-env-changed=MAENIFOLD_WEB_DIST");
    if std::env::var("MAENIFOLD_WEB_DIST").is_err() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let vendor_dir = manifest_dir
            .join("../../vendor/chrome-acp/packages/proxy-server/public");
        let fallback = if vendor_dir.join("index.html").exists() {
            vendor_dir
                .canonicalize()
                .expect("failed to canonicalize vendor web-dist path")
        } else {
            manifest_dir.join("web-dist")
        };
        println!("cargo:rustc-env=MAENIFOLD_WEB_DIST={}", fallback.display());
    }
}
