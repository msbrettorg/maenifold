#![allow(dead_code, unused_imports, clippy::duplicate_mod)]

/// Process-wide mutex for tests that mutate global Config state.
pub(crate) static GLOBAL_CONFIG_TEST_MUTEX: once_cell::sync::Lazy<parking_lot::Mutex<()>> =
    once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(()));

mod recent_activity_boundary_queries;
mod recent_activity_boundary_rendering;
mod recent_activity_pipeline_collect;
mod recent_activity_pipeline_format;
mod recent_activity_pipeline_helpers;
mod recent_activity_pipeline_tool;
mod recent_activity_tool_behavior;
