use crate::db::with_db;
use crate::types::{AppState, ChatCommandResult, Session, Workflow};
use serde_json::json;

pub(crate) fn handle_query_command(
    tokens: &[&str],
    state: &AppState,
    _app_session_id_ref: Option<&str>,
    result: &mut ChatCommandResult,
) -> Result<bool, String> {
    match tokens {
        ["/app_session", "list"] => {
            let sessions = with_db(state, |conn| {
                let mut stmt = conn
                    .prepare(
                        "SELECT id, workflow_id, status, initial_prompt, created_at, updated_at
                         FROM sessions ORDER BY created_at DESC LIMIT 50",
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
                let mut items = Vec::new();
                for row in rows {
                    items.push(row.map_err(|e| e.to_string())?);
                }
                Ok(items)
            })?;
            result.message = format!("{} sessions", sessions.len());
            result.payload = json!({ "sessions": sessions });
            Ok(true)
        }
        ["/app_workflow", "list"] => {
            let workflows = with_db(state, |conn| {
                let mut stmt = conn
                    .prepare(
                        "SELECT id, name, steps_json, created_at, updated_at
                         FROM workflows ORDER BY updated_at DESC",
                    )
                    .map_err(|e| e.to_string())?;
                let rows = stmt
                    .query_map([], |row| {
                        let steps_json: String = row.get(2)?;
                        let steps =
                            serde_json::from_str::<Vec<String>>(&steps_json).unwrap_or_default();
                        Ok(Workflow {
                            id: row.get(0)?,
                            name: row.get(1)?,
                            steps,
                            created_at: row.get(3)?,
                            updated_at: row.get(4)?,
                        })
                    })
                    .map_err(|e| e.to_string())?;
                let mut items = Vec::new();
                for row in rows {
                    items.push(row.map_err(|e| e.to_string())?);
                }
                Ok(items)
            })?;
            result.message = format!("{} workflows", workflows.len());
            result.payload = json!({ "workflows": workflows });
            Ok(true)
        }
        _ => Ok(false),
    }
}
