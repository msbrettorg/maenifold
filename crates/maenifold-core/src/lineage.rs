use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn stable_lineage_id(kind: &str, stable_key: &str) -> String {
    let normalized = stable_key.trim().to_ascii_lowercase();
    let mut hasher = Sha256::new();
    hasher.update(kind.as_bytes());
    hasher.update(b":");
    hasher.update(normalized.as_bytes());
    let digest = hasher.finalize();
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    format!("lin-v1-{kind}-{}", &hex[..20])
}

pub fn gate_task_lineage_id(task_id: &str) -> String {
    stable_lineage_id("gate-task", task_id)
}

pub fn pipeline_run_lineage_id(session_id: &str) -> String {
    stable_lineage_id("pipeline-run", session_id)
}

pub fn thinking_session_lineage_id(session_id: &str) -> String {
    stable_lineage_id("thinking-session", session_id)
}

pub fn memory_note_lineage_id(uri: &str) -> String {
    stable_lineage_id("memory-note", uri)
}

pub fn lineage_json(
    gate_task_id: Option<&str>,
    pipeline_run_id: Option<&str>,
    thinking_session_id: Option<&str>,
    memory_note_id: Option<&str>,
) -> Value {
    json!({
        "gateTaskId": gate_task_id,
        "pipelineRunId": pipeline_run_id,
        "thinkingSessionId": thinking_session_id,
        "memoryNoteId": memory_note_id,
    })
}
