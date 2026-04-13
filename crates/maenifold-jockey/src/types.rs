use crate::db::DbPool;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

pub struct AppState {
    pub(crate) db: DbPool,
    pub(crate) shared_context: DashMap<String, String>,
    /// In-memory cache for role rows; invalidated on upsert/delete.
    /// Wrapped in Arc so temporary AppState clones share the same cache.
    pub(crate) role_cache: Arc<DashMap<String, Arc<Role>>>,
}

impl AppState {
    /// Cheap clone of all shared handles — use when constructing short-lived
    /// temporary AppState instances for spawn_blocking tasks.
    pub(crate) fn clone_refs(&self) -> Self {
        Self {
            db: self.db.clone(),
            shared_context: self.shared_context.clone(),
            role_cache: self.role_cache.clone(),
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Role {
    pub(crate) id: String,
    pub(crate) role_name: String,
    pub(crate) runtime_kind: String,
    pub(crate) system_prompt: String,
    pub(crate) model: Option<String>,
    pub(crate) mode: Option<String>,
    pub(crate) mcp_servers_json: String,
    pub(crate) config_options_json: String,
    pub(crate) config_option_defs_json: String,
    pub(crate) auto_approve: bool,
    pub(crate) created_at: i64,
    pub(crate) updated_at: i64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Workflow {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) steps: Vec<String>,
    pub(crate) created_at: i64,
    pub(crate) updated_at: i64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Session {
    pub(crate) id: String,
    pub(crate) workflow_id: String,
    pub(crate) status: String,
    pub(crate) initial_prompt: String,
    pub(crate) created_at: i64,
    pub(crate) updated_at: i64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionEvent {
    pub(crate) id: i64,
    pub(crate) session_id: String,
    pub(crate) event_type: String,
    pub(crate) role_name: Option<String>,
    pub(crate) payload: Value,
    pub(crate) created_at: i64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContextEntry {
    pub(crate) scope: String,
    pub(crate) key: String,
    pub(crate) value: String,
    pub(crate) updated_at: i64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionUpdateEvent {
    pub(crate) session_id: String,
    pub(crate) workflow_id: String,
    pub(crate) role_name: String,
    pub(crate) delta: String,
    pub(crate) state: String,
    pub(crate) done: bool,
    pub(crate) created_at: i64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowStateEvent {
    pub(crate) session_id: String,
    pub(crate) workflow_id: String,
    pub(crate) status: String,
    pub(crate) active_role: Option<String>,
    pub(crate) message: String,
    pub(crate) created_at: i64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChatCommandResult {
    pub(crate) ok: bool,
    pub(crate) message: String,
    pub(crate) runtime_kind: Option<String>,
    pub(crate) session_id: Option<String>,
    pub(crate) payload: Value,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssistantRuntime {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) binary: String,
    pub(crate) available: bool,
    pub(crate) version: Option<String>,
    pub(crate) install_hint: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssistantChatInput {
    pub(crate) input: String,
    pub(crate) runtime_kind: Option<String>,
    pub(crate) app_session_id: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssistantChatResponse {
    pub(crate) ok: bool,
    pub(crate) reply: String,
    pub(crate) runtime_kind: Option<String>,
    pub(crate) session_id: Option<String>,
    pub(crate) command_result: Option<ChatCommandResult>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartWorkflowInput {
    pub(crate) workflow_id: String,
    pub(crate) initial_prompt: String,
}

#[derive(Default)]
pub(crate) struct ParsedRouteInput {
    pub(crate) role_names: Vec<String>,
    pub(crate) message: String,
    pub(crate) file_refs: Vec<String>,
    pub(crate) dir_refs: Vec<String>,
    pub(crate) skill_refs: Vec<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MentionCandidate {
    pub(crate) value: String,
    pub(crate) kind: String,
    pub(crate) detail: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppSession {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) active_role: String,
    pub(crate) runtime_kind: Option<String>,
    pub(crate) cwd: Option<String>,
    pub(crate) messages: Vec<serde_json::Value>,
    pub(crate) created_at: i64,
    pub(crate) last_active_at: i64,
    pub(crate) closed_at: Option<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppSessionUpdate {
    pub(crate) title: Option<String>,
    pub(crate) active_role: Option<String>,
    pub(crate) runtime_kind: Option<Option<String>>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppSkill {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) content: String,
    pub(crate) created_at: i64,
    pub(crate) updated_at: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppSkillUpsert {
    pub(crate) id: Option<String>,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) content: String,
}

pub(crate) const ATTACH_MAX_TOTAL_BYTES: usize = 160 * 1024;
pub(crate) const ATTACH_MAX_FILE_BYTES: usize = 24 * 1024;
pub(crate) const ATTACH_MAX_DIR_FILES: usize = 30;
pub(crate) const ATTACH_MAX_DIR_DEPTH: usize = 3;
pub(crate) const DEFAULT_MODELS: &[&str] = &[];
pub(crate) const KNOWN_RUNTIME_KEYS: &[&str] = &["gemini-cli", "claude-code", "codex-cli", "mock"];
pub(crate) const DEFAULT_MCP_SERVERS: &[&str] = &[];
pub(crate) const DEFAULT_SKILLS: &[&str] = &[];
pub(crate) const BASE_CLI_COMMANDS: &[(&str, &str)] = &[
    ("/app_help", "Show command help"),
    ("/app_cd", "Show current working directory"),
    ("/app_cd <path>", "Change working directory for all agents"),
    ("/app_assistant list", "List detected assistant runtimes"),
    (
        "/app_assistant select <runtime>",
        "Select active assistant runtime",
    ),
    ("/app_model list", "List configurable model catalog"),
    ("/app_model add <model>", "Add model to dynamic catalog"),
    (
        "/app_model remove <model>",
        "Remove model from dynamic catalog",
    ),
    ("/app_model select <model>", "Select assistant model"),
    (
        "/app_model select role <role> <model>",
        "Select model for a specific role",
    ),
    ("/app_model get", "Get selected assistant model"),
    ("/app_model clear", "Clear selected assistant model"),
    ("/app_mcp list", "List MCP catalog and enabled entries"),
    ("/app_mcp add <name>", "Add MCP server to catalog"),
    ("/app_mcp remove <name>", "Remove MCP server from catalog"),
    ("/app_mcp enable <name>", "Enable MCP server"),
    ("/app_mcp disable <name>", "Disable MCP server"),
    ("/app_role list", "List roles"),
    (
        "/app_role bind <role> <runtime> [prompt]",
        "Create or update role",
    ),
    (
        "/app_role prompt <role> <prompt>",
        "Update role system prompt",
    ),
    ("/app_context list", "List all shared context entries"),
    (
        "/app_context list <scope>",
        "List context entries for a scope",
    ),
    ("/app_session list", "List workflow sessions"),
    ("/app_workflow list", "List workflows"),
];
