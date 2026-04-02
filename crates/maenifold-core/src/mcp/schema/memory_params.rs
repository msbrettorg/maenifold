#![allow(dead_code)] // Schema-only types — fields are read by schemars, not by application code

use super::params::{
    default_one_u32, default_search_mode, default_ten_u32, default_true, default_zero_f64,
};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WriteMemoryParams {
    /// Title for this memory file
    pub title: String,
    /// Content with [[Concept Names]]; at least one [[concept]] is required
    pub content: String,
    /// Optional folder path for organizing files
    #[serde(default)]
    pub folder: Option<String>,
    /// Optional tags for categorizing this file
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadMemoryParams {
    /// Memory file identifier: URI or exact title
    pub identifier: String,
    /// Return checksum with content for safe editing
    #[serde(default = "default_true")]
    #[schemars(default = "default_true")]
    pub include_checksum: bool,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EditMemoryOperation {
    Append,
    Prepend,
    FindReplace,
    ReplaceSection,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditMemoryParams {
    /// Memory file identifier: URI or exact title
    pub identifier: String,
    /// Edit operation to apply
    pub operation: EditMemoryOperation,
    /// Content to add or replace; must include at least one [[concept]]
    pub content: String,
    /// File checksum from the last read for stale-edit protection
    #[serde(default)]
    pub checksum: Option<String>,
    /// Text to find when operation is find_replace
    #[serde(default)]
    pub find_text: Option<String>,
    /// Section name when operation is replace_section
    #[serde(default)]
    pub section_name: Option<String>,
    /// Expected match count when operation is find_replace
    #[serde(default)]
    pub expected_count: Option<u32>,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteMemoryParams {
    /// Memory file identifier: URI or exact title
    pub identifier: String,
    /// Confirm deletion of the file
    #[serde(default)]
    pub confirm: bool,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MoveMemoryParams {
    /// Source memory file identifier: URI or exact title
    pub source: String,
    /// Destination file name or relative path
    pub destination: String,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
pub enum SearchMode {
    Hybrid,
    Semantic,
    FullText,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchMemoriesParams {
    /// Search query over file contents
    pub query: String,
    /// Search mode
    #[serde(default = "default_search_mode")]
    #[schemars(default = "default_search_mode")]
    pub mode: SearchMode,
    /// Maximum files to return
    #[serde(default = "default_ten_u32")]
    #[schemars(default = "default_ten_u32")]
    pub page_size: u32,
    /// Page number for results
    #[serde(default = "default_one_u32")]
    #[schemars(default = "default_one_u32")]
    pub page: u32,
    /// Optional folder path filter under the memory root
    #[serde(default)]
    pub folder: Option<String>,
    /// Optional tag filter; all provided tags must match
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Minimum score threshold
    #[serde(default = "default_zero_f64")]
    #[schemars(default = "default_zero_f64")]
    pub min_score: f64,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtractConceptsFromFileParams {
    /// Memory file identifier: URI or exact title
    pub identifier: String,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListMemoriesParams {
    /// Optional directory path relative to the memory root
    #[serde(default)]
    pub path: Option<String>,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}
