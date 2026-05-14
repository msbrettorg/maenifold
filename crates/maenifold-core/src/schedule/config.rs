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
    #[serde(default)]
    pub auto_grant_permissions: bool,
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    #[serde(default)]
    pub mcp_action: Option<String>,
    #[serde(default)]
    pub mcp_parameters: Option<serde_json::Value>,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u32,
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

#[cfg(test)]
mod tests {
    use super::ScheduledTask;
    use serde_json::json;

    #[test]
    fn missing_permission_fields_default_to_safe_values() {
        let task = serde_json::from_value::<ScheduledTask>(json!({
            "id": "legacy",
            "name": "Legacy task",
            "schedule": "0 9 * * *",
            "agent": "swe",
            "prompt": "run legacy task",
            "created_at": "2026-04-21T07:00:00Z",
            "updated_at": "2026-04-21T07:00:00Z"
        }))
        .expect("legacy task should deserialize");

        assert!(!task.auto_grant_permissions);
        assert!(task.allowed_tools.is_empty());
    }

    #[test]
    fn schedule_task_serde_round_trip_with_mcp_action() {
        let task = serde_json::from_value::<ScheduledTask>(json!({
            "id": "mcp",
            "name": "MCP task",
            "schedule": "0 9 * * *",
            "agent": "swe",
            "prompt": "",
            "mcp_action": "maenifold.gate.gate_Status",
            "mcp_parameters": {"root": "/workspace"},
            "created_at": "2026-04-21T07:00:00Z",
            "updated_at": "2026-04-21T07:00:00Z"
        }))
        .expect("task with mcp action should deserialize");
        let rendered = serde_json::to_value(&task).expect("serialize task");

        assert_eq!(rendered["mcp_action"], "maenifold.gate.gate_Status");
        assert_eq!(rendered["mcp_parameters"], json!({"root": "/workspace"}));
    }

    #[test]
    fn schedule_task_serde_default_mcp_action_none() {
        let task = serde_json::from_value::<ScheduledTask>(json!({
            "id": "legacy-mcp",
            "name": "Legacy MCP task",
            "schedule": "0 9 * * *",
            "agent": "swe",
            "prompt": "run legacy task",
            "created_at": "2026-04-21T07:00:00Z",
            "updated_at": "2026-04-21T07:00:00Z"
        }))
        .expect("legacy task should deserialize");
        let rendered = serde_json::to_value(&task).expect("serialize task");

        assert_eq!(rendered["mcp_action"], serde_json::Value::Null);
        assert_eq!(rendered["mcp_parameters"], serde_json::Value::Null);
    }
}
