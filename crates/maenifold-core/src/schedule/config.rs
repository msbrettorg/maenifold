use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: String,
    pub name: String,
    pub schedule: String,
    #[serde(default)]
    pub schedule_human: Option<String>,
    pub agent: String,
    pub prompt: String,
    #[serde(default)]
    pub workdir: Option<PathBuf>,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u32,
    #[serde(default = "default_true")]
    pub skip_if_running: bool,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub last_run: Option<RunRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRecord {
    pub status: RunStatus,
    pub timestamp: DateTime<Utc>,
    pub duration_seconds: u64,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunStatus {
    Success,
    Failed,
    Timeout,
    Skipped,
}

fn default_timeout() -> u32 {
    300
}

fn default_true() -> bool {
    true
}
