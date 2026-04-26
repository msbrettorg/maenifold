mod acp_exec;
mod acp_exec_context;
mod acp_exec_payload;
mod catalog;
mod context;
mod diagnostics;
mod embedding_catalog;
mod embedding_download;
pub(crate) mod embedding_models;
mod embedding_models_actions;
mod embedding_models_detect;
mod embedding_models_scan;
mod read_text_file;
mod recent_activity;
mod recent_activity_collect;
mod recent_activity_format;
mod recent_activity_render;
mod response;
mod schedule;
mod schedule_logs;
mod sessions;
mod sessions_query;
mod teams_monitor;
mod teams_monitor_auth;
mod teams_monitor_helpers;
mod wallpaper_clear_cache;
mod wallpaper_status;

pub use acp_exec::acp_exec;
pub use catalog::{list_assets, read_mcp_resource};
pub use context::{ToolContext, not_yet_implemented, render};
pub use diagnostics::{get_config, memory_status};
pub use embedding_models::embedding_models;
pub use read_text_file::read_text_file;
pub use recent_activity::{
    RecentActivityRecord, collect_recent_activity_records, parse_recent_activity_timespan,
    recent_activity,
};
pub use recent_activity_format::{
    format_recent_activity_report, is_valid_session_id_format, recent_activity_determine_file_type,
    recent_activity_extract_session_id,
};
pub use recent_activity_render::recent_activity_extract_snippet;
pub use schedule::schedule;
pub use sessions::list_sessions;
pub use teams_monitor::teams_monitor;
pub use wallpaper_clear_cache::wallpaper_clear_cache;
pub use wallpaper_status::wallpaper_status;
// T-13.5: RTM FR-8 — ToolResponse helpers ported from C# Utils/ToolResponse.cs
pub use response::{with_hint, with_next_steps};
