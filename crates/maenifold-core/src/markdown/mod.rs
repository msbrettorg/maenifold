mod filter;
mod paths;
mod storage;
mod wikilinks;

// T-12.0: RTM NFR-1 preserves the `markdown` facade while delegating to focused submodules.
pub use filter::WikiLinkFilter;
pub use paths::{
    delete_file, file_exists, find_file_as_uri, get_memory_files, markdown_path_from_relative,
    path_to_uri, resolve_folder_uri, uri_to_path, validate_uri,
};
pub(crate) use storage::drain_frontmatter_warnings;
pub use storage::{
    Frontmatter, append_h2_section, generate_checksum, read_markdown, update_markdown,
    write_markdown,
};
pub use wikilinks::{
    extract_h2_sections, extract_wikilinks, extract_wikilinks_with_sections, normalize_concept,
    slugify, strip_wikilink_brackets,
};
