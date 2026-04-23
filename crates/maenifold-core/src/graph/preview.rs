// T-01.16: Facade for the graph::preview module.
//
// Implementation lives in sibling submodules; this file only re-exports the
// public surface so external call sites (graph::buildcontext, etc.) continue
// to use `preview::create_smart_preview`, `preview::extract_section_with_concept`,
// `preview::parse_source_files_for_json`, and `preview::load_content_preview_map`
// without modification.

mod matcher;
mod section;
mod source_files;
mod text_utils;

#[cfg(test)]
mod tests;

pub use section::{create_smart_preview, extract_section_with_concept};
pub(crate) use source_files::{load_content_preview_map, parse_source_files_for_json};
