use std::fs;
use std::path::{Path, PathBuf};

// T-12.6: RTM FR-5
pub(crate) fn load_help_doc(assets_path: &Path, tool_name: &str) -> Option<String> {
    for root in [Some(assets_path.to_path_buf()), packaged_assets_path()]
        .into_iter()
        .flatten()
    {
        let path = root
            .join("usage")
            .join("tools")
            .join(format!("{tool_name}.md"));
        if let Ok(content) = fs::read_to_string(path) {
            return Some(content);
        }
    }
    None
}

// T-9.5: RTM FR-9 / RTM-065 — resolve assets at runtime so release binaries
// work without the build-machine path baked in by env!("CARGO_MANIFEST_DIR").
fn packaged_assets_path() -> Option<PathBuf> {
    // 1. exe-adjacent (release installs)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let candidate = exe_dir.join("assets");
            if candidate.is_dir() {
                return Some(candidate);
            }
        }
    }
    // 2. dev mode: walk up from exe to workspace root
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent();
        while let Some(d) = dir {
            if d.join("Cargo.toml").exists() && d.join("assets").is_dir() {
                return Some(d.join("assets"));
            }
            dir = d.parent();
        }
    }
    None
}
