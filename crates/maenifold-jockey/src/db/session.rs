use super::{parse_payload, with_db};
use crate::db::context::{list_shared_context_internal, set_shared_context_internal};
use crate::db::role::{load_role, load_role_runtime_kind};
use crate::db::workflow::load_workflow;
use crate::emitter::Emitter;
use crate::types::*;
use crate::{acp, default_chat_cwd, now_ms};
use rusqlite::params;
use serde_json::json;
use uuid::Uuid;

pub(crate) fn record_session_event(
    state: &AppState,
    session_id: &str,
    event_type: &str,
    role_name: Option<&str>,
    payload: serde_json::Value,
) -> Result<SessionEvent, String> {
    let now = now_ms();
    let payload_text = payload.to_string();
    let id = with_db(state, |conn| {
        conn.execute(
            "INSERT INTO session_events (session_id, event_type, role_name, payload, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![session_id, event_type, role_name, payload_text, now],
        )
        .map_err(|e| e.to_string())?;
        Ok(conn.last_insert_rowid())
    })?;
    Ok(SessionEvent {
        id,
        session_id: session_id.to_string(),
        event_type: event_type.to_string(),
        role_name: role_name.map(|s| s.to_string()),
        payload,
        created_at: now,
    })
}

/// Insert multiple session events in a single connection + transaction.
/// Returns the last inserted row id (or 0 if the slice is empty).
pub(crate) fn record_session_events_batch(
    state: &AppState,
    events: &[(&str, &str, Option<&str>, serde_json::Value)], // (session_id, event_type, role_name, payload)
) -> Result<(), String> {
    if events.is_empty() {
        return Ok(());
    }
    let now = now_ms();
    with_db(state, |conn| {
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        for (session_id, event_type, role_name, payload) in events {
            let payload_text = payload.to_string();
            tx.execute(
                "INSERT INTO session_events (session_id, event_type, role_name, payload, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![session_id, event_type, role_name, payload_text, now],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())
    })
}

pub(crate) fn update_session_status(
    state: &AppState,
    session_id: &str,
    status: &str,
) -> Result<(), String> {
    let now = now_ms();
    with_db(state, |conn| {
        conn.execute(
            "UPDATE sessions SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status, now, session_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
}

pub(crate) fn list_sessions(state: &AppState) -> Result<Vec<Session>, String> {
    with_db(state, |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, workflow_id, status, initial_prompt, created_at, updated_at
                 FROM sessions ORDER BY created_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Session {
                    id: row.get(0)?,
                    workflow_id: row.get(1)?,
                    status: row.get(2)?,
                    initial_prompt: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row.map_err(|e| e.to_string())?);
        }
        Ok(sessions)
    })
}

fn session_event_from_row(row: &rusqlite::Row) -> Result<SessionEvent, String> {
    let payload_text: String = row.get(4).map_err(|e| e.to_string())?;
    Ok(SessionEvent {
        id: row.get(0).map_err(|e| e.to_string())?,
        session_id: row.get(1).map_err(|e| e.to_string())?,
        event_type: row.get(2).map_err(|e| e.to_string())?,
        role_name: row.get(3).map_err(|e| e.to_string())?,
        payload: parse_payload(payload_text),
        created_at: row.get(5).map_err(|e| e.to_string())?,
    })
}

pub(crate) fn list_session_events(
    state: &AppState,
    session_id: String,
    cursor: Option<i64>,
    limit: Option<u32>,
) -> Result<Vec<SessionEvent>, String> {
    let bounded = limit.unwrap_or(200).min(1000);
    with_db(state, |conn| {
        let mut events = Vec::new();
        if let Some(c) = cursor {
            let mut stmt = conn
                .prepare(
                    "SELECT id, session_id, event_type, role_name, payload, created_at
                     FROM session_events
                     WHERE session_id = ?1 AND id > ?2
                     ORDER BY id ASC
                     LIMIT ?3",
                )
                .map_err(|e| e.to_string())?;
            let mut rows = stmt
                .query(params![&session_id, c, bounded])
                .map_err(|e| e.to_string())?;
            while let Some(row) = rows.next().map_err(|e| e.to_string())? {
                events.push(session_event_from_row(row)?);
            }
        } else {
            let mut stmt = conn
                .prepare(
                    "SELECT id, session_id, event_type, role_name, payload, created_at
                     FROM session_events
                     WHERE session_id = ?1
                     ORDER BY id ASC
                     LIMIT ?2",
                )
                .map_err(|e| e.to_string())?;
            let mut rows = stmt
                .query(params![&session_id, bounded])
                .map_err(|e| e.to_string())?;
            while let Some(row) = rows.next().map_err(|e| e.to_string())? {
                events.push(session_event_from_row(row)?);
            }
        }
        Ok(events)
    })
}

pub(crate) fn summarize_text(input: &str) -> String {
    let normalized = input.replace('\n', " ");
    let trimmed = normalized.trim();
    if trimmed.chars().count() <= 180 {
        return trimmed.to_string();
    }
    trimmed.chars().take(180).collect()
}

pub(crate) fn chunk_text(input: &str, size: usize) -> Vec<String> {
    if input.is_empty() || size == 0 {
        return vec![];
    }
    let mut chunks = Vec::new();
    let mut buf = String::new();
    let mut count = 0usize;
    for ch in input.chars() {
        buf.push(ch);
        count += 1;
        if count >= size {
            chunks.push(std::mem::take(&mut buf));
            count = 0;
        }
    }
    if !buf.is_empty() {
        chunks.push(buf);
    }
    chunks
}

pub(crate) async fn run_workflow(
    event_tx: crate::EventSender,
    session_id: String,
    workflow: Workflow,
    seed_prompt: String,
) -> Result<(), String> {
    // TODO: pass AppState explicitly
    let state: &AppState = todo!();
    let workspace_path = default_chat_cwd();
    let context_scope = format!("workflow:{session_id}");
    let mut prompt = seed_prompt;
    for (step_idx, role_name) in workflow.steps.iter().enumerate() {
        let runtime_kind =
            load_role_runtime_kind(state, role_name).unwrap_or_else(|_| "mock".to_string());

        if let Some(next_role) = workflow.steps.get(step_idx + 1) {
            let next_runtime =
                load_role_runtime_kind(state, next_role).unwrap_or_else(|_| "mock".to_string());
            let next_role = next_role.clone();
            let next_cwd = workspace_path.clone();
            tokio::spawn(async move {
                acp::prewarm_role(&next_runtime, &next_role, &next_cwd, None).await;
            });
        }
        let started = WorkflowStateEvent {
            session_id: session_id.clone(),
            workflow_id: workflow.id.clone(),
            status: "running".to_string(),
            active_role: Some(role_name.clone()),
            message: format!("{role_name} started"),
            created_at: now_ms(),
        };
        event_tx
            .emit("workflow/state_changed", started.clone())
            .map_err(|e| e.to_string())?;
        // StepStarted is written individually so it's visible before ACP runs
        record_session_event(
            state,
            &session_id,
            "StepStarted",
            Some(role_name),
            json!({ "message": started.message, "runtimeKind": runtime_kind }),
        )?;

        let context_entries = list_shared_context_internal(state, &context_scope)?;
        let context_pairs = context_entries
            .iter()
            .map(|entry| (entry.key.clone(), entry.value.clone()))
            .collect::<Vec<_>>();
        let role_data = load_role(state, role_name).unwrap_or(None);
        let auto_approve = role_data.as_ref().map(|r| r.auto_approve).unwrap_or(true);
        let role_mode = role_data.as_ref().and_then(|r| r.mode.clone());
        let role_config: Vec<(String, String)> = role_data
            .as_ref()
            .and_then(|r| serde_json::from_str::<serde_json::Value>(&r.config_options_json).ok())
            .and_then(|v| {
                v.as_object().map(|m| {
                    m.iter()
                        .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                        .collect()
                })
            })
            .unwrap_or_default();
        let acp_result = acp::execute_runtime(
            &runtime_kind,
            role_name,
            &prompt,
            &context_pairs,
            &workspace_path,
            &event_tx,
            auto_approve,
            vec![],
            role_mode,
            role_config,
            None,
            "",
        )
        .await;
        let output = acp_result.output;
        let chunks = if acp_result.deltas.is_empty() {
            chunk_text(&output, 30)
        } else {
            acp_result.deltas
        };

        for chunk in &chunks {
            let update = SessionUpdateEvent {
                session_id: session_id.clone(),
                workflow_id: workflow.id.clone(),
                role_name: role_name.clone(),
                delta: chunk.clone(),
                state: "writing".to_string(),
                done: false,
                created_at: now_ms(),
            };
            event_tx
                .emit("session/update", update)
                .map_err(|e| e.to_string())?;
        }

        let summary = summarize_text(&output);
        set_shared_context_internal(
            state,
            &context_scope,
            &format!("summary.{role_name}"),
            &summary,
        )?;
        // Batch AdapterResult + StepOutput + StepCompleted into one transaction
        record_session_events_batch(
            state,
            &[
                (
                    &session_id,
                    "AdapterResult",
                    Some(role_name.as_str()),
                    acp_result.meta.clone(),
                ),
                (
                    &session_id,
                    "StepOutput",
                    Some(role_name.as_str()),
                    json!({ "output": &output }),
                ),
                (
                    &session_id,
                    "StepCompleted",
                    Some(role_name.as_str()),
                    json!({ "summary": &summary }),
                ),
            ],
        )?;

        prompt = format!("{}\n\n{} handoff summary: {}", prompt, role_name, summary);
        let done_update = SessionUpdateEvent {
            session_id: session_id.clone(),
            workflow_id: workflow.id.clone(),
            role_name: role_name.clone(),
            delta: "".to_string(),
            state: "idle".to_string(),
            done: true,
            created_at: now_ms(),
        };
        event_tx
            .emit("session/update", done_update)
            .map_err(|e| e.to_string())?;
    }

    update_session_status(state, &session_id, "completed")?;
    let completed = WorkflowStateEvent {
        session_id: session_id.clone(),
        workflow_id: workflow.id.clone(),
        status: "completed".to_string(),
        active_role: workflow.steps.last().cloned(),
        message: "workflow completed".to_string(),
        created_at: now_ms(),
    };
    event_tx
        .emit("workflow/state_changed", completed.clone())
        .map_err(|e| e.to_string())?;
    record_session_event(
        state,
        &session_id,
        "WorkflowCompleted",
        workflow.steps.last().map(String::as_str),
        json!({ "message": completed.message }),
    )?;
    Ok(())
}

pub(crate) async fn start_workflow(
    event_tx: crate::EventSender,
    state: &AppState,
    input: StartWorkflowInput,
) -> Result<Session, String> {
    let workflow = load_workflow(state, &input.workflow_id)?;
    if workflow.steps.is_empty() {
        return Err("workflow has no steps".to_string());
    }

    let session_id = Uuid::new_v4().to_string();
    let now = now_ms();

    with_db(state, |conn| {
        conn.execute(
            "INSERT INTO sessions (id, workflow_id, status, initial_prompt, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                &session_id,
                &input.workflow_id,
                "running",
                &input.initial_prompt,
                now,
                now,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })?;

    let session = Session {
        id: session_id.clone(),
        workflow_id: input.workflow_id.clone(),
        status: "running".to_string(),
        initial_prompt: input.initial_prompt.clone(),
        created_at: now,
        updated_at: now,
    };

    record_session_event(
        state,
        &session_id,
        "WorkflowStarted",
        None,
        json!({ "prompt": session.initial_prompt }),
    )?;

    let workflow_notice = WorkflowStateEvent {
        session_id: session_id.clone(),
        workflow_id: session.workflow_id.clone(),
        status: "running".to_string(),
        active_role: workflow.steps.first().cloned(),
        message: "workflow started".to_string(),
        created_at: now_ms(),
    };
    event_tx
        .emit("workflow/state_changed", workflow_notice)
        .map_err(|e| e.to_string())?;

    let event_tx_clone = event_tx.clone();
    let workflow_copy = workflow.clone();
    let session_id_copy = session_id.clone();
    let prompt_copy = input.initial_prompt;
    let state = state.clone_refs();

    tokio::spawn(async move {
        let result = run_workflow(
            event_tx_clone.clone(),
            session_id_copy.clone(),
            workflow_copy,
            prompt_copy,
        )
        .await;
        if let Err(error) = result {
            let _ = update_session_status(&state, &session_id_copy, "error");
            let wf_id = {
                with_db(&state, |conn| {
                    conn.query_row(
                        "SELECT workflow_id FROM sessions WHERE id = ?1",
                        params![&session_id_copy],
                        |row| row.get::<_, String>(0),
                    )
                    .map_err(|e| e.to_string())
                })
                .unwrap_or_default()
            };
            let failed = WorkflowStateEvent {
                session_id: session_id_copy.clone(),
                workflow_id: wf_id,
                status: "error".to_string(),
                active_role: None,
                message: error.clone(),
                created_at: now_ms(),
            };
            let _ = event_tx_clone.emit("workflow/state_changed", failed.clone());
            let _ = record_session_event(
                &state,
                &session_id_copy,
                "WorkflowFailed",
                None,
                json!({ "error": failed.message }),
            );
        }
    });

    Ok(session)
}
