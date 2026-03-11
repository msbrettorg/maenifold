mod api;
mod display;
mod text;
// T-12.0: RTM NFR-1 preserve the search facade while moving vector helpers under search/.
mod vector;
#[allow(unused_imports)]
pub use vector::*;

// T-12.7: RTM NFR-1 preserves the public [[search]] facade while the implementation is split into focused submodules.
pub use crate::graph::{
    build_context, create_smart_preview, extract_section_with_concept, random_walk, visualize,
};
pub use crate::repair::extract_concepts_from_file;
pub use crate::similar::find_similar_concepts;
pub use crate::sync::{
    VectorFailureReasonCodes, VectorSyncDiagnostics, VectorUnavailableReasonCodes,
    build_vector_failure_report, build_vector_unavailable_report, generate_concept_embeddings,
    generate_file_embedding, recompute_communities_if_needed, sync, sync_files, sync_single_file,
};
pub use api::search_memories;
pub use display::get_file_display_info;
