use crate::assets::support::{load_help_doc as support_load_help_doc, resolve_packaged_assets_root};
use std::path::Path;

// T-12.6: RTM FR-5
pub(crate) fn load_help_doc(assets_path: &Path, tool_name: &str) -> Option<String> {
    for root in [Some(assets_path.to_path_buf()), resolve_packaged_assets_root()]
        .into_iter()
        .flatten()
    {
        if let Some(content) = support_load_help_doc(&root, tool_name) {
            return Some(content);
        }
    }
    None
}
