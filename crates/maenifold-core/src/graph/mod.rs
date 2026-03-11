use crate::error::Result;
use crate::tools::ToolContext;
use rusqlite::Connection;
use serde_json::Value;

pub mod analyzer;
mod build_context;
mod formatter;
pub mod preview;
mod support;
mod traversal;

// T-12.0: RTM NFR-1 preserves the graph facade while directory submodules own the implementation split.
pub use build_context::build_context;
pub(crate) use support::{
    emit_graph_tools_diagnostic, format_co_occurrence_weight, load_help_doc,
    relation_weighted_score,
};
pub(crate) use traversal::{
    CommunitySiblingLine, GraphRelation, RelatedConceptLine, load_community_siblings,
    load_concept_community_id, load_graph_neighbors,
};

// T-12.0: RTM NFR-1 keeps the legacy preview facade stable while graph/preview.rs owns the implementation.
pub fn create_smart_preview(content: &str, target_length: usize, tolerance: usize) -> String {
    preview::create_smart_preview(content, target_length, tolerance)
}

// T-12.0: RTM NFR-1 preserves the legacy section-extraction entrypoint during the split.
pub fn extract_section_with_concept(
    content: &str,
    concept_name: &str,
    max_length: usize,
) -> String {
    preview::extract_section_with_concept(content, concept_name, max_length)
}

// T-12.0: RTM NFR-1 preserves the legacy random-walk facade while graph/analyzer.rs owns traversal internals.
pub fn random_walk(
    connection: &Connection,
    start_concept: &str,
    walk_length: Option<usize>,
) -> Result<Vec<String>> {
    analyzer::random_walk(connection, start_concept, walk_length)
}

// T-12.0: RTM NFR-1 preserves the Mermaid visualization facade while graph/analyzer.rs owns the implementation.
pub fn visualize(context: &ToolContext, payload: &Value) -> Result<String> {
    analyzer::visualize(context, payload)
}

// T-12.0: RTM NFR-1 preserves the similarity-search facade while similar.rs owns the implementation.
pub fn find_similar_concepts(context: &ToolContext, payload: &Value) -> Result<String> {
    crate::similar::find_similar_concepts(context, payload)
}
