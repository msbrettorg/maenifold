use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::Deserialize;

use crate::cli::{GateContext, StatusArgs};
use crate::gate_tool_service::GateToolService;
use crate::status_filter::StatusFilter;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub(crate) struct TaskEntry {
    pub(crate) task_id: String,
    pub(crate) title: String,
    pub(crate) pipeline: Option<String>,
    pub(crate) state: String,
    #[serde(default)]
    pub(crate) stages: Vec<String>,
    pub(crate) location: String,
    pub(crate) attempts: u32,
    pub(crate) path: PathBuf,
    #[serde(default)]
    pub(crate) issue: String,
}

#[derive(Debug, Clone)]
pub(crate) struct TaskDetail {
    pub(crate) entry: TaskEntry,
    pub(crate) markdown: String,
}

pub(crate) fn load_tasks(root: &Path) -> Result<Vec<TaskEntry>, String> {
    let value = GateToolService::status(StatusArgs {
        context: GateContext {
            root: root.to_path_buf(),
            requested_pipeline_name: None,
            json_output: false,
        },
        filter: StatusFilter {
            state: None,
            ready_only: false,
            include_done: false,
        },
    })
    .map_err(|e| e.to_string())?;

    let entries: Vec<TaskEntry> =
        serde_json::from_value(value).map_err(|e| format!("failed to parse task data: {e}"))?;
    Ok(entries)
}

pub(crate) fn read_model_timestamp(root: &Path) -> Option<SystemTime> {
    latest_modified_time([
        root.join(".gate").join("gate.db"),
        root.join(".gate").join("gate.db-wal"),
    ])
}

pub(crate) fn latest_modified_time(paths: [PathBuf; 2]) -> Option<SystemTime> {
    paths
        .into_iter()
        .filter_map(|path| fs::metadata(path).ok()?.modified().ok())
        .fold(None, |latest, modified| match latest {
            Some(current) if current >= modified => Some(current),
            _ => Some(modified),
        })
}
