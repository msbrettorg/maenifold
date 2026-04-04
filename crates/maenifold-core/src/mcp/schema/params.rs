#![allow(dead_code)] // Schema-only types — fields are read by schemars, not by application code

pub(super) fn default_true() -> bool {
    true
}
pub(super) fn default_zero_u32() -> u32 {
    0
}
pub(super) fn default_one_u32() -> u32 {
    1
}
pub(super) fn default_two_u32() -> u32 {
    2
}
pub(super) fn default_five_u32() -> u32 {
    5
}
pub(super) fn default_eight_u32() -> u32 {
    8
}
pub(super) fn default_ten_u32() -> u32 {
    10
}
pub(super) fn default_twenty_u32() -> u32 {
    20
}
pub(super) fn default_thirty_u32() -> u32 {
    30
}
pub(super) fn default_fifty_u32() -> u32 {
    50
}
pub(super) fn default_one_hundred_fifty_u32() -> u32 {
    150
}
pub(super) fn default_one_thousand_u32() -> u32 {
    1000
}
pub(super) fn default_zero_f64() -> f64 {
    0.0
}
pub(super) fn default_point_seven() -> f64 {
    0.7
}
pub(super) fn default_search_mode() -> super::memory_params::SearchMode {
    super::memory_params::SearchMode::Hybrid
}
pub(super) fn default_recent_activity_filter()
-> super::thinking_workflow_params::RecentActivityFilter {
    super::thinking_workflow_params::RecentActivityFilter::All
}
pub(super) fn default_edge_type() -> super::pipeline_params::PipelineEdgeType {
    super::pipeline_params::PipelineEdgeType::Success
}
pub(super) fn default_prompt_mode() -> Option<String> {
    Some("prompt".to_string())
}

pub use super::assumption_params::*;
pub use super::graph_params::*;
pub use super::memory_params::*;
pub use super::pipeline_params::*;
pub use super::system_params::*;
pub use super::thinking_workflow_params::*;
