// Out-of-band compile guard. See AGENTS.md > Build.
//
// The only approved build entry points are the rebuild scripts:
//   macOS/Linux : ./rebuild.sh
//   Windows     : pwsh -NoProfile -File .\rebuild.ps1
// Both export MAENIFOLD_REBUILD=1 before invoking cargo. Any compile without
// that token is "out of band" and is rejected here.
//
// rust-analyzer's build-script / proc-macro pre-pass is exempt (it exports
// RA_RUSTC_WRAPPER=1) so editor/LSP intelligence keeps working.
fn main() {
    println!("cargo:rerun-if-env-changed=MAENIFOLD_REBUILD");
    println!("cargo:rerun-if-env-changed=RA_RUSTC_WRAPPER");

    let approved = std::env::var_os("MAENIFOLD_REBUILD").is_some();
    let rust_analyzer = std::env::var_os("RA_RUSTC_WRAPPER").is_some();
    if approved || rust_analyzer {
        return;
    }

    eprintln!();
    eprintln!("error: out-of-band compile blocked by the maenifold build guard.");
    eprintln!("       The only approved build path is the rebuild script:");
    eprintln!("         macOS/Linux : ./rebuild.sh");
    eprintln!("         Windows     : pwsh -NoProfile -File .\\rebuild.ps1");
    eprintln!("       Direct `cargo build` / `cargo test` / `cargo check` is forbidden.");
    eprintln!("       See AGENTS.md > Build.");
    eprintln!();
    std::process::exit(1);
}
