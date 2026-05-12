use agent_client_protocol as acp;
use serde::Serialize;
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};

// ── Event types broadcast to the frontend ────────────────────────────────────

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionDeathEvent {
    pub runtime_key: String,
    pub role_name: String,
    pub app_session_id: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrewarmEvent {
    pub runtime_key: String,
    pub role_name: String,
    pub app_session_id: String,
    pub status: PrewarmStatus,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PrewarmStatus {
    Started,
    Ready,
    Failed { error: String },
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcpPromptResult {
    pub ok: bool,
    pub output: String,
    pub error_code: Option<String>,
    pub deltas: Vec<String>,
    pub meta: Value,
}

#[derive(Debug)]
pub struct AcpExecuteSuccess {
    pub output: String,
    pub session_id: String,
    pub resumed: bool,
}

#[derive(Debug)]
pub struct AcpExecuteError {
    pub message: String,
    pub requested_resume: bool,
    pub resumed: bool,
    pub resume_failure_kind: Option<String>,
}

pub type AcpExecuteResult = Result<AcpExecuteSuccess, AcpExecuteError>;

#[derive(Clone, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AcpEvent {
    UserTextDelta {
        text: String,
    },
    TextDelta {
        text: String,
    },
    ThoughtDelta {
        text: String,
    },
    ToolCall {
        tool_call_id: String,
        title: String,
        tool_kind: String,
        status: String,
        content: Option<Vec<Value>>,
        locations: Option<Vec<Value>>,
        raw_input: Option<Value>,
        raw_output: Option<Value>,
    },
    ToolCallUpdate {
        tool_call_id: String,
        tool_kind: Option<String>,
        status: Option<String>,
        title: Option<String>,
        content: Option<Vec<Value>>,
        locations: Option<Vec<Value>>,
        raw_input: Option<Value>,
        raw_output: Option<Value>,
    },
    Plan {
        entries: Vec<Value>,
    },
    PermissionRequest {
        request_id: String,
        title: String,
        description: Option<String>,
        options: Vec<Value>,
    },
    ModeUpdate {
        mode_id: String,
    },
    ConfigUpdate {
        options: Vec<Value>,
    },
    SessionInfo {
        title: Option<String>,
    },
    StatusUpdate {
        text: String,
    },
    AvailableCommands {
        commands: Vec<Value>,
    },
    AvailableModes {
        modes: Vec<Value>,
        current: Option<String>,
    },
    PermissionExpired {
        request_id: String,
    },
    ReplayComplete,
}

// ── Worker message channel ────────────────────────────────────────────────────

pub enum WorkerMsg {
    Execute {
        runtime_key: String,
        binary: String,
        args: Vec<String>,
        env: Vec<(String, String)>,
        client_capabilities: acp::ClientCapabilities,
        role_name: String,
        app_session_id: String,
        prompt: String,
        context: Vec<(String, String)>,
        prompt_blocks: Option<Vec<acp::ContentBlock>>,
        cwd: String,
        delta_tx: mpsc::Sender<AcpEvent>,
        result_tx: oneshot::Sender<AcpExecuteResult>,
        auto_approve: bool,
        mcp_servers: Vec<acp::McpServer>,
        role_mode: Option<String>,
        role_config_options: Vec<(String, String)>,
        resume_session_id: Option<String>,
    },
    Prewarm {
        runtime_key: String,
        binary: String,
        args: Vec<String>,
        env: Vec<(String, String)>,
        client_capabilities: acp::ClientCapabilities,
        role_name: String,
        app_session_id: String,
        cwd: String,
        auto_approve: bool,
        mcp_servers: Vec<acp::McpServer>,
        role_mode: Option<String>,
        role_config_options: Vec<Value>,
        delta_tx: Option<mpsc::Sender<AcpEvent>>,
        result_tx: Option<oneshot::Sender<(Vec<Value>, Vec<String>, String)>>,
        resume_session_id: Option<String>,
    },
    Cancel {
        runtime_key: String,
        role_name: String,
        app_session_id: String,
    },
    Reset {
        runtime_key: String,
        role_name: String,
        app_session_id: String,
        result_tx: oneshot::Sender<Result<(), String>>,
    },
    Reconnect {
        runtime_key: String,
        role_name: String,
        app_session_id: String,
        result_tx: oneshot::Sender<Result<(), String>>,
    },
    GetResumeState {
        runtime_key: String,
        role_name: String,
        app_session_id: String,
        result_tx: oneshot::Sender<bool>,
    },
    SetMode {
        runtime_key: String,
        role_name: String,
        app_session_id: String,
        mode_id: String,
        result_tx: oneshot::Sender<Result<(), String>>,
    },
    SetConfigOption {
        runtime_key: String,
        role_name: String,
        app_session_id: String,
        config_id: String,
        value: String,
        result_tx: oneshot::Sender<Result<(), String>>,
    },
    CancelTrackedSession {
        session_id: String,
        result_tx: oneshot::Sender<Result<super::pool::TrackedSessionReap, String>>,
    },
    ListLiveSessions {
        result_tx: oneshot::Sender<Vec<String>>,
    },
    Shutdown {
        done_tx: oneshot::Sender<()>,
    },
}
