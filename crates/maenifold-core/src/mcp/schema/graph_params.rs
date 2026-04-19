#![allow(dead_code)] // Schema-only types — fields are read by schemars, not by application code

use super::params::{
    default_fifty_u32, default_point_seven, default_ten_u32, default_thirty_u32, default_true,
    default_twenty_u32, default_two_u32,
};
use schemars::JsonSchema;
use serde::Deserialize;

fn default_visualize_format() -> VisualizeFormat {
    VisualizeFormat::Mermaid
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncParams {
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuildContextParams {
    /// Concept name to build context around
    pub concept_name: String,
    /// How many graph hops to traverse
    #[serde(default = "default_two_u32")]
    #[schemars(default = "default_two_u32")]
    pub depth: u32,
    /// Maximum related entities to return
    #[serde(default = "default_twenty_u32")]
    #[schemars(default = "default_twenty_u32")]
    pub max_entities: u32,
    /// Include content previews in the context response
    #[serde(default)]
    pub include_content: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualizeParams {
    /// Central concept for the visualization
    pub concept_name: String,
    /// Graph traversal depth
    #[serde(default = "default_two_u32")]
    #[schemars(default = "default_two_u32")]
    pub depth: u32,
    /// Maximum nodes to include in the diagram
    #[serde(default = "default_thirty_u32")]
    #[schemars(default = "default_thirty_u32")]
    pub max_nodes: u32,
    /// Visualization output format
    #[serde(default = "default_visualize_format")]
    #[schemars(default = "default_visualize_format")]
    pub format: VisualizeFormat,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum VisualizeFormat {
    Mermaid,
    Graph,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RepairConceptsParams {
    /// Delimited concept variants to replace
    pub concepts_to_replace: String,
    /// Canonical concept to keep after consolidation
    pub canonical_concept: String,
    /// Optional folder under the memory root to process
    #[serde(default)]
    pub folder: Option<String>,
    /// Preview changes without modifying files
    #[serde(default = "default_true")]
    #[schemars(default = "default_true")]
    pub dry_run: bool,
    /// Create WikiLinks from plain text instead of replacing existing ones
    #[serde(default)]
    pub create_wiki_links: bool,
    /// Minimum semantic similarity required for safe consolidation
    #[serde(default = "default_point_seven")]
    #[schemars(default = "default_point_seven")]
    pub min_semantic_similarity: f64,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnalyzeConceptCorruptionParams {
    /// Concept family to analyze, such as tool or agent
    pub concept_family: String,
    /// Maximum variants to include
    #[serde(default = "default_fifty_u32")]
    #[schemars(default = "default_fifty_u32")]
    pub max_results: u32,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FindSimilarConceptsParams {
    /// Concept name to search for related concepts
    pub concept_name: String,
    /// Maximum number of similar concepts to return
    #[serde(default = "default_ten_u32")]
    #[schemars(default = "default_ten_u32")]
    pub max_results: u32,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}
