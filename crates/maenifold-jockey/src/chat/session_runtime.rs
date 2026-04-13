use crate::db::app_session_role::load_app_session_role_state;
use crate::db::context::{list_shared_context_internal, sanitize_dynamic_item_name};
use crate::db::role::load_role;
use crate::db::session_context::app_session_role_scope;
use crate::runtime_kind::RuntimeKind;
use crate::types::AppState;

use super::RecentRoleChat;

pub(super) struct RoleRuntimeData {
    pub(super) runtime: String,
    pub(super) context_pairs: Vec<(String, String)>,
    pub(super) auto_approve: bool,
    pub(super) role_mode: Option<String>,
    pub(super) role_config: Vec<(String, String)>,
    pub(super) role_system_prompt: Option<String>,
    pub(super) context_log: Option<(usize, Option<String>)>,
    pub(super) mcp_servers: Vec<agent_client_protocol::McpServer>,
}

fn upsert_context_pair(context_pairs: &mut Vec<(String, String)>, key: &str, value: String) {
    if let Some(existing) = context_pairs.iter_mut().find(|(k, _)| k == key) {
        existing.1 = value;
    } else {
        context_pairs.push((key.to_string(), value));
    }
}

fn parse_config_map(raw: &str) -> Vec<(String, String)> {
    serde_json::from_str::<serde_json::Value>(raw)
        .ok()
        .and_then(|v| {
            v.as_object().map(|m| {
                m.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                    .collect()
            })
        })
        .unwrap_or_default()
}

fn normalize_runtime_key(runtime: &str) -> String {
    RuntimeKind::from_str(runtime)
        .map(|k| k.runtime_key().to_string())
        .unwrap_or_else(|| runtime.trim().to_ascii_lowercase())
}

fn is_claude_family_alias(model_lc: &str) -> bool {
    matches!(model_lc, "sonnet" | "haiku" | "opus") || model_lc.starts_with("claude-")
}

fn pick_canonical_discovered_model(
    runtime_key: &str,
    selected_model: &str,
    discovered: &[String],
) -> String {
    let selected = selected_model.trim();
    if selected.is_empty() {
        return String::new();
    }
    let selected_lc = selected.to_ascii_lowercase();
    if runtime_key != "claude-code" && is_claude_family_alias(&selected_lc) {
        return String::new();
    }
    if discovered.is_empty() {
        return selected.to_string();
    }

    if runtime_key == "claude-code" && matches!(selected_lc.as_str(), "sonnet" | "haiku" | "opus") {
        let mut candidates: Vec<&String> = discovered
            .iter()
            .filter(|m| {
                let m_lc = m.to_ascii_lowercase();
                m_lc.starts_with("claude-")
                    && (m_lc.contains(&format!("-{}-", selected_lc))
                        || m_lc.ends_with(&format!("-{}", selected_lc)))
            })
            .collect();
        if !candidates.is_empty() {
            candidates.sort_by(|a, b| a.cmp(b));
            if let Some(best) = candidates.last() {
                return (*best).clone();
            }
        }
    }

    if let Some(exact) = discovered
        .iter()
        .find(|m| m.trim().eq_ignore_ascii_case(selected))
    {
        return exact.clone();
    }

    let mut fuzzy: Vec<&String> = discovered
        .iter()
        .filter(|m| m.to_ascii_lowercase().contains(&selected_lc))
        .collect();
    if fuzzy.is_empty() {
        return selected.to_string();
    }
    fuzzy.sort_by(|a, b| {
        let a_lc = a.to_ascii_lowercase();
        let b_lc = b.to_ascii_lowercase();
        let a_score = (a_lc.starts_with("claude-") as u8, a_lc.len());
        let b_score = (b_lc.starts_with("claude-") as u8, b_lc.len());
        a_score.cmp(&b_score).then_with(|| a.cmp(b))
    });
    fuzzy
        .last()
        .map(|m| (*m).clone())
        .unwrap_or_else(|| selected.to_string())
}

fn normalize_model_for_runtime(runtime: &str, selected_model: &str) -> String {
    let normalized_runtime = RuntimeKind::from_str(runtime)
        .map(|k| k.runtime_key())
        .unwrap_or(runtime);
    let discovered = crate::acp::list_discovered_models(normalized_runtime);
    pick_canonical_discovered_model(normalized_runtime, selected_model, &discovered)
}

fn sanitize_model_for_runtime(runtime: &str, selected_model: &str) -> Option<String> {
    let normalized = normalize_model_for_runtime(runtime, selected_model);
    let trimmed = normalized.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn resolve_model(
    role_data: Option<&crate::types::Role>,
    role_state: Option<&crate::db::app_session_role::AppSessionRoleState>,
    runtime_key: &str,
    runtime: &str,
    context_pairs: &[(String, String)],
) -> Option<String> {
    let state_runtime_matches = |r: &crate::db::app_session_role::AppSessionRoleState| -> bool {
        let sr = r.runtime_kind.as_deref().map(normalize_runtime_key);
        sr.as_deref() == Some(runtime_key) || sr.is_none()
    };

    let model_override = role_state
        .filter(|r| {
            r.runtime_kind
                .as_deref()
                .map(normalize_runtime_key)
                .as_deref()
                == Some(runtime_key)
        })
        .and_then(|r| r.model_override.clone());

    if let Some(model) = model_override.and_then(|m| sanitize_model_for_runtime(runtime, &m)) {
        return Some(model);
    }

    let session_cfg_model = role_state
        .filter(|r| state_runtime_matches(r))
        .and_then(|r| r.config_options_json.as_deref())
        .and_then(|raw| {
            parse_config_map(raw)
                .into_iter()
                .find(|(k, _)| k == "model")
                .map(|(_, v)| v)
        });
    if let Some(model) = session_cfg_model.and_then(|m| sanitize_model_for_runtime(runtime, &m)) {
        return Some(model);
    }

    let role_cfg_model = role_data
        .map(|r| parse_config_map(&r.config_options_json))
        .and_then(|cfg| cfg.into_iter().find(|(k, _)| k == "model").map(|(_, v)| v));
    if let Some(model) = role_cfg_model.and_then(|m| sanitize_model_for_runtime(runtime, &m)) {
        return Some(model);
    }

    let role_model = role_data.and_then(|r| r.model.clone());
    if let Some(model) = role_model.and_then(|m| sanitize_model_for_runtime(runtime, &m)) {
        return Some(model);
    }

    context_pairs
        .iter()
        .find(|(k, _)| k == "model")
        .and_then(|(_, v)| sanitize_model_for_runtime(runtime, v))
}

pub(super) fn load_role_runtime_data(
    state: &AppState,
    app_session_id: &str,
    role_name: &str,
    assistant_runtime: &str,
    recent_chats_snapshot: Vec<RecentRoleChat>,
) -> Result<RoleRuntimeData, String> {
    let role_state = load_app_session_role_state(state, app_session_id, role_name)?;
    let role_data = load_role(state, role_name)?;

    let runtime = if role_name == "Jockey" {
        assistant_runtime.to_string()
    } else {
        if role_state.is_none() && role_data.is_none() {
            return Err(format!("role not found: {role_name}"));
        }
        role_state
            .as_ref()
            .and_then(|row| row.runtime_kind.clone())
            .or_else(|| role_data.as_ref().map(|r| r.runtime_kind.clone()))
            .ok_or_else(|| format!("runtime not found for role: {role_name}"))?
    };

    let scope = app_session_role_scope(app_session_id, role_name);
    let entries = list_shared_context_internal(state, &scope).unwrap_or_default();
    let mut context_pairs: Vec<(String, String)> =
        entries.into_iter().map(|e| (e.key, e.value)).collect();

    let mut context_log = None;

    if role_name != "Jockey" {
        let role_prompt = role_data
            .as_ref()
            .map(|r| r.system_prompt.clone())
            .unwrap_or_default();

        if !role_prompt.is_empty() {
            context_pairs.push(("role_prompt".to_string(), role_prompt));
        }
        // Only inject cross-role context on the first message to this role in the session.
        // If this role has already replied at least once, it already has its own history
        // in the ACP session — no need to keep prepending the handoff context every turn.
        let this_role_has_history = recent_chats_snapshot.iter().any(|c| c.role == role_name);

        let cross_role_chats: Vec<_> = recent_chats_snapshot
            .into_iter()
            .filter(|c| c.role != role_name)
            .collect();

        let inherited_cwd: Option<String> = cross_role_chats
            .iter()
            .rev()
            .find(|c| !c.cwd.is_empty())
            .map(|c| c.cwd.clone());

        if !cross_role_chats.is_empty() && !this_role_has_history {
            if let Ok(payload) = serde_json::to_string(&cross_role_chats) {
                upsert_context_pair(&mut context_pairs, "from_last_role_context", payload);
            }
            context_log = Some((cross_role_chats.len(), inherited_cwd.clone()));
        }
        if let Some(prev_cwd) = inherited_cwd {
            upsert_context_pair(&mut context_pairs, "cwd", prev_cwd);
        }
    }
    let auto_approve = role_data.as_ref().map(|r| r.auto_approve).unwrap_or(true);
    let runtime_key = normalize_runtime_key(&runtime);
    let role_mode = role_state
        .as_ref()
        .and_then(|r| {
            let state_runtime = r.runtime_kind.as_deref().map(normalize_runtime_key);
            if state_runtime.as_deref() == Some(runtime_key.as_str()) || state_runtime.is_none() {
                r.mode_override.clone()
            } else {
                None
            }
        })
        .or_else(|| role_data.as_ref().and_then(|r| r.mode.clone()));
    let mut role_config: Vec<(String, String)> = role_data
        .as_ref()
        .map(|r| parse_config_map(&r.config_options_json))
        .unwrap_or_default();
    if let Some(session_cfg) = role_state.as_ref().and_then(|r| {
        let state_runtime = r.runtime_kind.as_deref().map(normalize_runtime_key);
        if state_runtime.as_deref() == Some(runtime_key.as_str()) || state_runtime.is_none() {
            r.config_options_json.as_deref()
        } else {
            None
        }
    }) {
        for (key, value) in parse_config_map(session_cfg) {
            if value.trim().is_empty() {
                role_config.retain(|(k, _)| k != &key);
                continue;
            }
            if let Some(existing) = role_config.iter_mut().find(|(k, _)| k == &key) {
                existing.1 = value;
            } else {
                role_config.push((key, value));
            }
        }
    }

    role_config.retain(|(k, _)| k != "model");
    if let Some(model) = resolve_model(
        role_data.as_ref(),
        role_state.as_ref(),
        &runtime_key,
        &runtime,
        &context_pairs,
    ) {
        role_config.push(("model".to_string(), model));
    }

    let role_system_prompt = role_data
        .as_ref()
        .map(|r| r.system_prompt.clone())
        .filter(|s| !s.is_empty());
    let mut mcp_servers: Vec<agent_client_protocol::McpServer> = {
        let mut servers = crate::db::global_mcp::load_all_global_mcp_as_acp(state);
        let role_servers = role_data
            .as_ref()
            .map(|r| &r.mcp_servers_json)
            .map(|raw| crate::db::global_mcp::parse_mcp_server_list_json_compat(raw))
            .unwrap_or_default();
        let existing_names: std::collections::HashSet<String> = servers
            .iter()
            .map(|s| match s {
                agent_client_protocol::McpServer::Http(h) => h.name.clone(),
                agent_client_protocol::McpServer::Sse(e) => e.name.clone(),
                agent_client_protocol::McpServer::Stdio(d) => d.name.clone(),
                _ => String::new(),
            })
            .collect();
        for s in role_servers {
            let name = match &s {
                agent_client_protocol::McpServer::Http(h) => h.name.as_str(),
                agent_client_protocol::McpServer::Sse(e) => e.name.as_str(),
                agent_client_protocol::McpServer::Stdio(d) => d.name.as_str(),
                _ => "",
            };
            if !name.is_empty() && !existing_names.contains(name) {
                servers.push(s);
            }
        }
        servers
    };

    // Respect per-session MCP feature flags when present (mcp:<name>=enabled/disabled).
    // If no flag is set for this scope, keep default role/session MCP server list.
    let mcp_flags: std::collections::HashMap<String, String> = context_pairs
        .iter()
        .filter_map(|(k, v)| {
            k.strip_prefix("mcp:")
                .map(|name| (name.to_ascii_lowercase(), v.to_ascii_lowercase()))
        })
        .collect();
    if !mcp_flags.is_empty() {
        mcp_servers.retain(|server| {
            let server_name = match server {
                agent_client_protocol::McpServer::Http(s) => s.name.as_str(),
                agent_client_protocol::McpServer::Sse(s) => s.name.as_str(),
                agent_client_protocol::McpServer::Stdio(s) => s.name.as_str(),
                _ => "",
            };
            let normalized_name = sanitize_dynamic_item_name(server_name)
                .unwrap_or_else(|| server_name.trim().to_ascii_lowercase());
            match mcp_flags.get(&normalized_name) {
                Some(flag) => flag == "enabled",
                None => true,
            }
        });
    }

    let mcp_names: Vec<&str> = mcp_servers
        .iter()
        .map(|s| match s {
            agent_client_protocol::McpServer::Http(h) => h.name.as_str(),
            agent_client_protocol::McpServer::Sse(e) => e.name.as_str(),
            agent_client_protocol::McpServer::Stdio(d) => d.name.as_str(),
            _ => "",
        })
        .filter(|n| !n.is_empty())
        .collect();
    let meta_header = format!(
        "[Jockey context]\nrole: {role_name}\nruntime: {runtime}\nmcp_servers: {mcp}\n\nWhen calling jockey MCP tools that require a roleName parameter, use \"{role_name}\" unless the user specifies otherwise.",
        mcp = if mcp_names.is_empty() { "none".to_string() } else { mcp_names.join(", ") },
    );
    let role_system_prompt = Some(match role_system_prompt {
        Some(sp) => format!("{meta_header}\n\n{sp}"),
        None => meta_header,
    });

    Ok(RoleRuntimeData {
        runtime,
        context_pairs,
        auto_approve,
        role_mode,
        role_config,
        role_system_prompt,
        context_log,
        mcp_servers,
    })
}
