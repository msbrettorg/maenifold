mod analysis;
mod analysis_helpers;
mod api;
mod semantics;
mod wikilinks;

// T-3.1: RTM FR-6
pub use analysis::{analyze_concept_corruption, extract_concepts_from_file};
// T-3.1: RTM FR-6
pub use api::repair_concepts;

// T-3.1: RTM FR-6
pub(crate) use semantics::{
    embedding_is_valid_for_similarity_search, is_vec_embedding_unavailable, load_concept_embedding,
};
// T-3.1: RTM FR-6
pub(crate) use wikilinks::split_frontmatter_for_repair;
