pub mod acp;
mod db;
pub mod emitter;
pub mod types;

/// Type alias for the event broadcast sender used throughout the crate.
pub type EventSender = tokio::sync::broadcast::Sender<JockeyEvent>;

/// Events emitted by the ACP bridge.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum JockeyEvent {
    AcpDelta(serde_json::Value),
    AcpStream(serde_json::Value),
    AcpConnectionLost(serde_json::Value),
    AcpPrewarm(serde_json::Value),
    CommandApplied(serde_json::Value),
    WorkflowStateChanged(serde_json::Value),
    SessionUpdate(serde_json::Value),
    McpError(String),
}
