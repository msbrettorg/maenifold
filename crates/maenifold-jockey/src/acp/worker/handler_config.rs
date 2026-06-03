use agent_client_protocol::schema::{self as acp};
use serde_json::{Value, json};
use tokio::sync::mpsc;

use super::super::adapter::acp_log;
use super::super::runtime_state::list_discovered_config_options;
use super::notify::notify_connection_death;
use super::pool::{CANCEL_HANDLES, CONN_MAP};
use super::types::{AcpEvent, ConnectionDeathEvent};

/// Keys whose failure to set should be treated as fatal for prompt execution.
/// If a pipeline node explicitly requests a model or thought level and the
/// agent rejects it, we must not silently fall back to the agent's default.
const CRITICAL_CONFIG_KEYS: &[&str] = &["model", "thoughtLevel", "thought_level"];

fn is_critical_config_key(key: &str) -> bool {
    CRITICAL_CONFIG_KEYS.contains(&key)
}

/// Apply config options after a cold-start (or warm re-use).
///
/// Returns `Err` if a *critical* config option (`model`, `thoughtLevel`,
/// `thought_level`) fails to apply.  Non-critical option failures are logged
/// but tolerated so the prompt can still proceed.
pub(crate) async fn apply_cold_start_config(
    key: &str,
    session_id: &acp::SessionId,
    _delta_tx: &mpsc::Sender<AcpEvent>,
    role_config_options: &[(String, String)],
    runtime_key: &str,
) -> Result<(), String> {
    let conn_rc = CONN_MAP.with(|m| m.borrow().get(key).map(|c| c.conn.clone()));
    let Some(conn_rc) = conn_rc else {
        return Ok(());
    };

    let discovered = list_discovered_config_options(runtime_key);
    let supported_keys: std::collections::HashSet<String> = discovered
        .iter()
        .filter_map(|v| v.get("id").and_then(|i| i.as_str()).map(|s| s.to_string()))
        .collect();

    for (k, value) in role_config_options {
        if !supported_keys.is_empty() && !supported_keys.contains(k) {
            if is_critical_config_key(k) {
                let msg = format!(
                    "Agent does not support config option '{k}' — cannot guarantee requested model/thoughtLevel"
                );
                acp_log(
                    "config.set_option.critical_unsupported",
                    json!({ "key": k, "runtime": runtime_key, "reason": "critical key not supported by runtime" }),
                );
                return Err(msg);
            }
            acp_log(
                "config.set_option.skipped",
                json!({ "key": k, "runtime": runtime_key, "reason": "not supported by runtime" }),
            );
            continue;
        }
        if let Err(e) = conn_rc
            .send_request(acp::SetSessionConfigOptionRequest::new(
                session_id.clone(),
                acp::SessionConfigId::from(k.clone()),
                acp::SessionConfigValueId::from(value.clone()),
            ))
            .block_task()
            .await
        {
            if is_critical_config_key(k) {
                let msg = format!("Failed to set {k} '{value}' on agent session: {e}");
                acp_log(
                    "config.set_option.critical_error",
                    json!({ "key": k, "value": value, "error": e.to_string() }),
                );
                return Err(msg);
            }
            acp_log(
                "config.set_option.error",
                json!({ "key": k, "error": e.to_string() }),
            );
        }
    }
    Ok(())
}

pub(crate) fn structured_config_option_pairs(
    role_config_options: &[Value],
    runtime_key: &str,
) -> Vec<(String, String)> {
    role_config_options
        .iter()
        .filter_map(|option| {
            let Some(key) = option.get("id").and_then(Value::as_str) else {
                acp_log(
                    "config.set_option.skipped",
                    json!({ "runtime": runtime_key, "reason": "missing id", "option": option }),
                );
                return None;
            };
            let Some(value) = option.get("currentValue").and_then(|current| match current {
                Value::String(value) => Some(value.clone()),
                Value::Bool(value) => Some(value.to_string()),
                Value::Number(value) => Some(value.to_string()),
                _ => None,
            }) else {
                acp_log(
                    "config.set_option.skipped",
                    json!({ "key": key, "runtime": runtime_key, "reason": "missing scalar currentValue" }),
                );
                return None;
            };
            Some((key.to_string(), value))
        })
        .collect()
}

pub(crate) fn build_prompt_blocks(
    prompt: String,
    context: &[(String, String)],
) -> Vec<acp::ContentBlock> {
    let mut blocks: Vec<acp::ContentBlock> =
        vec![acp::ContentBlock::Text(acp::TextContent::new(prompt))];
    if !context.is_empty() {
        let ctx = context
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("\n");
        blocks.push(acp::ContentBlock::Text(acp::TextContent::new(format!(
            "[context]\n{ctx}"
        ))));
    }
    blocks
}

/// Spawn a task that watches the health channel and evicts the connection when
/// the agent process dies.
pub(crate) fn spawn_connection_health_watch(
    key: String,
    instance_id: u64,
    death: ConnectionDeathEvent,
    mut hrx: tokio::sync::watch::Receiver<bool>,
) {
    tokio::task::spawn_local(async move {
        loop {
            if hrx.changed().await.is_err() {
                break;
            }
            if !*hrx.borrow() {
                break;
            }
        }
        acp_log("health.process_died", json!({ "key": &key }));

        // Only evict if the connection in the map is still the same instance.
        let is_same = CONN_MAP.with(|m| {
            m.borrow()
                .get(&key)
                .map(|c| c.instance_id == instance_id)
                .unwrap_or(false)
        });

        if is_same {
            CONN_MAP.with(|m| m.borrow_mut().remove(&key));
            CANCEL_HANDLES.with(|m| m.borrow_mut().remove(&key));
            notify_connection_death(death);
        }
    });
}
