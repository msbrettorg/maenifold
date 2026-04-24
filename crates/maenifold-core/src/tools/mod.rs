mod acp_exec;
mod acp_exec_context;
mod acp_exec_payload;
mod catalog;
mod context;
mod diagnostics;
mod embedding_download;
mod embedding_models;
mod embedding_models_actions;
mod embedding_models_detect;
mod embedding_models_scan;
mod recent_activity;
mod recent_activity_collect;
mod recent_activity_format;
mod recent_activity_render;
mod response;
mod sessions;
mod sessions_query;

pub use acp_exec::acp_exec;
pub use catalog::{list_assets, read_mcp_resource};
pub use context::{ToolContext, not_yet_implemented, render};
pub use diagnostics::{get_config, memory_status};
pub use embedding_models::embedding_models;
pub use recent_activity::{
    RecentActivityRecord, collect_recent_activity_records, parse_recent_activity_timespan,
    recent_activity,
};
pub use recent_activity_format::{
    format_recent_activity_report, is_valid_session_id_format, recent_activity_determine_file_type,
    recent_activity_extract_session_id,
};
pub use recent_activity_render::recent_activity_extract_snippet;
pub use sessions::list_sessions;
// T-13.5: RTM FR-8 — ToolResponse helpers ported from C# Utils/ToolResponse.cs
pub use response::{with_hint, with_next_steps};
