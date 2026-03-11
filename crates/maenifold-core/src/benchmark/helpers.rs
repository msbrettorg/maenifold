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

fn packaged_assets_path() -> Option<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(|root| root.join("assets"))
}
