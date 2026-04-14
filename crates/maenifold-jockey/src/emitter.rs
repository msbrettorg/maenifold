use serde::Serialize;

use crate::JockeyEvent;

/// Compatibility shim replacing `tauri::Emitter`.
/// Implements the same `emit(event, payload)` pattern but routes to a broadcast channel.
pub trait Emitter {
    fn emit<S: Serialize + Clone>(&self, event: &str, payload: S) -> Result<(), String>;
}

impl Emitter for tokio::sync::broadcast::Sender<JockeyEvent> {
    fn emit<S: Serialize + Clone>(&self, event: &str, payload: S) -> Result<(), String> {
        let value = serde_json::to_value(&payload).unwrap_or_default();
        let jockey_event = match event {
            "acp/delta" => JockeyEvent::AcpDelta(value),
            "acp/stream" => JockeyEvent::AcpStream(value),
            "acp/connection-lost" => JockeyEvent::AcpConnectionLost(value),
            "acp/prewarm" => JockeyEvent::AcpPrewarm(value),
            "command/applied" => JockeyEvent::CommandApplied(value),
            "workflow/state_changed" => JockeyEvent::WorkflowStateChanged(value),
            "session/update" => JockeyEvent::SessionUpdate(value),
            other => {
                JockeyEvent::AcpStream(serde_json::json!({ "event": other, "payload": value }))
            }
        };
        let _ = self.send(jockey_event);
        Ok(())
    }
}
