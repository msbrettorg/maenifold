mod context_bundle;
mod prompt_builder;
pub(crate) mod session_runtime;

use crate::chat::session_runtime::load_role_runtime_data;
use crate::commands::apply_chat_command;
use crate::db::context::{list_shared_context_internal, set_shared_context_internal};
use crate::db::session_context::app_session_scope;
use crate::parser::parse_route_input;
use crate::types::*;
use crate::{acp, build_jockey_tool_prompt, clip_text, now_ms};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Instant;

const RECENT_ROLE_CHATS_KEY: &str = "recentRoleChats";
const RECENT_ROLE_CHATS_LIMIT: usize = 9;
const RECENT_ROLE_TURNS_PER_ROLE: usize = 3;
const RECENT_ROLE_CHAT_TEXT_MAX: usize = 5000;

pub(crate) fn chat_log(event: &str, payload: serde_json::Value) {
    eprintln!("[jockey.chat] {} {} {}", now_ms(), event, payload);
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(super) struct RecentRoleChat {
    role: String,
    user: String,
    assistant: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    cwd: String,
}

fn collapse_whitespace(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut in_space = true;
    for ch in raw.chars() {
        if ch.is_whitespace() {
            if !in_space {
                out.push(' ');
                in_space = true;
            }
        } else {
            out.push(ch);
            in_space = false;
        }
    }
    if out.ends_with(' ') {
        out.pop();
    }
    out
}

fn normalize_recent_chat_text(raw: &str) -> String {
    let compact = collapse_whitespace(raw.trim());
    clip_text(&compact, RECENT_ROLE_CHAT_TEXT_MAX)
}

pub(super) fn load_recent_role_chats(
    state: &AppState,
    app_session_id: &str,
) -> Vec<RecentRoleChat> {
    let scope = app_session_scope(app_session_id);
    let entries = list_shared_context_internal(state, &scope).unwrap_or_default();
    entries
        .into_iter()
        .find(|entry| entry.key == RECENT_ROLE_CHATS_KEY)
        .and_then(|entry| serde_json::from_str::<Vec<RecentRoleChat>>(&entry.value).ok())
        .unwrap_or_default()
}

fn append_recent_role_chat(
    state: &AppState,
    role_name: &str,
    user: &str,
    assistant: &str,
    cwd: &str,
    app_session_id: &str,
) {
    let mut chats = load_recent_role_chats(state, app_session_id);
    // Trim this role's history to keep only the most recent (TURNS_PER_ROLE - 1)
    // entries, then append the new one — so at most TURNS_PER_ROLE per role.
    let mut role_count = chats.iter().filter(|c| c.role == role_name).count();
    while role_count >= RECENT_ROLE_TURNS_PER_ROLE {
        if let Some(pos) = chats.iter().position(|c| c.role == role_name) {
            chats.remove(pos);
            role_count -= 1;
        } else {
            break;
        }
    }
    chats.push(RecentRoleChat {
        role: role_name.to_string(),
        user: normalize_recent_chat_text(user),
        assistant: normalize_recent_chat_text(assistant),
        cwd: cwd.to_string(),
    });
    // Global cap across all roles.
    if chats.len() > RECENT_ROLE_CHATS_LIMIT {
        let drop_count = chats.len() - RECENT_ROLE_CHATS_LIMIT;
        chats.drain(0..drop_count);
    }
    if let Ok(payload) = serde_json::to_string(&chats) {
        let scope = app_session_scope(app_session_id);
        let _ = set_shared_context_internal(state, &scope, RECENT_ROLE_CHATS_KEY, &payload);
    }
}

pub(crate) fn detect_reply_signals(reply: &str) -> Vec<String> {
    let text = reply.to_ascii_lowercase();
    let mut signals = Vec::new();
    if text.contains("memory show")
        || text.contains("memory refresh")
        || text.contains("memory add")
        || text.contains("memory list")
    {
        signals.push("memory".to_string());
    }
    if text.contains("available_commands_update") {
        signals.push("available_commands_update".to_string());
    }
    if text.contains("acp") {
        signals.push("acp".to_string());
    }
    signals
}

pub(crate) fn extract_command_output(reply: &str) -> Option<String> {
    let trimmed = reply.trim();
    if trimmed.starts_with('/') {
        return Some(trimmed.to_string());
    }
    if trimmed.starts_with('`') && trimmed.ends_with('`') {
        let inner = trimmed.trim_matches('`').trim();
        if inner.starts_with('/') {
            return Some(inner.to_string());
        }
    }
    for line in trimmed.lines() {
        let cleaned = line.trim().trim_matches('`').trim();
        if cleaned.starts_with('/') {
            return Some(cleaned.to_string());
        }
    }
    None
}

pub(crate) async fn assistant_chat(
    event_tx: crate::EventSender,
    state: &AppState,
    input: AssistantChatInput,
) -> Result<AssistantChatResponse, String> {
    let started = Instant::now();
    let text = input.input.trim().to_string();
    let app_session_id = input
        .app_session_id
        .clone()
        .filter(|sid| !sid.trim().is_empty())
        .ok_or_else(|| "app session id required".to_string())?;
    chat_log(
        "request.start",
        json!({
            "inputSize": text.len(),
            "preview": clip_text(&text, 120),
            "runtimeKind": input.runtime_kind.clone()
        }),
    );
    if text.is_empty() {
        chat_log(
            "request.empty",
            json!({
                "latencyMs": started.elapsed().as_millis()
            }),
        );
        return Ok(AssistantChatResponse {
            ok: false,
            reply: "empty input".to_string(),
            runtime_kind: input.runtime_kind,
            session_id: None,
            command_result: None,
        });
    }

    if text.starts_with("/app_") {
        let route_started = Instant::now();
        let command_result = apply_chat_command(
            event_tx.clone(),
            state,
            text,
            input.runtime_kind.clone(),
            Some(app_session_id.clone()),
        )
        .await?;
        chat_log(
            "route.command",
            json!({
                "latencyMs": route_started.elapsed().as_millis(),
                "totalLatencyMs": started.elapsed().as_millis(),
                "ok": command_result.ok,
                "message": clip_text(&command_result.message, 120)
            }),
        );
        return Ok(AssistantChatResponse {
            ok: command_result.ok,
            reply: command_result.message.clone(),
            runtime_kind: command_result.runtime_kind.clone(),
            session_id: command_result.session_id.clone(),
            command_result: Some(command_result),
        });
    }

    let assistant = input
        .runtime_kind
        .clone()
        .ok_or_else(|| "assistant not selected".to_string())?;

    let routed = parse_route_input(&text);
    let explicit_role_targets = !routed.role_names.is_empty();
    let mut role_targets = if explicit_role_targets {
        routed.role_names.clone()
    } else {
        vec!["Jockey".to_string()]
    };
    if role_targets.is_empty() {
        role_targets.push("Jockey".to_string());
    }
    let mut message = routed.message.clone();
    if message.is_empty() {
        message = "Please answer based on the attached context.".to_string();
    }

    let tool_prompt = build_jockey_tool_prompt();
    let bundle = context_bundle::build_context_bundle(&state, &app_session_id, &routed).await;
    let cwd = bundle.cwd.clone();
    let attachment_pairs = bundle.attachment_pairs;
    let attach_notes = bundle.attach_notes;
    let skill_pairs = bundle.skill_pairs;
    let all_recent_chats = bundle.recent_chats;
    let mut role_outputs: Vec<(String, String)> = Vec::new();
    let mut any_acp_error = false;

    for role_name in role_targets {
        let is_union_assistant = role_name == "Jockey";
        let tmp_state = state.clone_refs();
        let role_name_clone = role_name.clone();
        let assistant_clone = assistant.clone();
        let app_session_id_clone = app_session_id.clone();
        let recent_chats_snapshot = all_recent_chats.clone();
        let db_data = tokio::task::spawn_blocking(move || {
            load_role_runtime_data(
                &tmp_state,
                &app_session_id_clone,
                &role_name_clone,
                &assistant_clone,
                recent_chats_snapshot,
            )
        })
        .await
        .map_err(|e| e.to_string())??;

        let runtime = db_data.runtime;
        let mut context_pairs = db_data.context_pairs;
        let auto_approve = db_data.auto_approve;
        let role_mode = db_data.role_mode;
        let role_config = db_data.role_config;
        let role_system_prompt = db_data.role_system_prompt;
        let mcp_servers = db_data.mcp_servers;

        if let Some((count, inherited_cwd)) = db_data.context_log {
            chat_log(
                "route.context.share",
                json!({
                    "role": role_name.clone(),
                    "recentChatCount": count,
                    "inheritedCwd": inherited_cwd
                }),
            );
        }

        context_pairs.extend(attachment_pairs.iter().cloned());
        context_pairs.extend(skill_pairs.iter().cloned());
        if !attach_notes.is_empty() {
            context_pairs.push(("attachment_notes".to_string(), attach_notes.join("\n")));
        }
        if !context_pairs.iter().any(|(k, _)| k == "cwd") {
            context_pairs.insert(0, ("cwd".to_string(), cwd.clone()));
        }

        let prepared = prompt_builder::build_prepared_prompt(
            is_union_assistant,
            &tool_prompt,
            role_system_prompt.as_deref(),
            &context_pairs,
            &message,
        );

        chat_log(
            "route.acp.start",
            json!({
                "runtime": runtime.clone(),
                "role": role_name.clone(),
                "cwd": cwd.clone(),
                "contextCount": context_pairs.len(),
                "preparedSize": prepared.len()
            }),
        );
        let acp_started = Instant::now();
        let llm = acp::execute_runtime(
            &runtime,
            &role_name,
            &prepared,
            &[],
            &cwd,
            &event_tx,
            auto_approve,
            mcp_servers,
            role_mode,
            role_config,
            Some((state, &app_session_id)),
            &app_session_id,
        )
        .await;
        let output = llm.output.trim().to_string();
        let llm_delta_count = llm.deltas.len();
        let llm_meta = llm.meta.clone();
        chat_log(
            "route.acp.done",
            json!({
                "runtime": runtime.clone(),
                "role": role_name.clone(),
                "latencyMs": acp_started.elapsed().as_millis(),
                "totalLatencyMs": started.elapsed().as_millis(),
                "outputSize": output.len(),
                "deltaCount": llm_delta_count,
                "meta": llm_meta
            }),
        );
        let signals = detect_reply_signals(&output);
        if !signals.is_empty() {
            chat_log(
                "route.acp.signal",
                json!({
                    "signals": signals,
                    "preview": clip_text(&output, 220)
                }),
            );
        }

        let mut final_output = output;
        if !explicit_role_targets && is_union_assistant {
            if let Some(command_text) = extract_command_output(&final_output) {
                chat_log(
                    "route.acp.command_output.suggested",
                    json!({
                        "command": clip_text(&command_text, 180)
                    }),
                );
            }
        }
        final_output = prompt_builder::with_command_suggestion(
            final_output,
            explicit_role_targets,
            is_union_assistant,
        );
        if !is_union_assistant {
            append_recent_role_chat(
                state,
                &role_name,
                &message,
                &final_output,
                &cwd,
                &app_session_id,
            );
        }
        if !llm.ok {
            any_acp_error = true;
        }
        role_outputs.push((role_name.clone(), final_output));
    }

    let reply = if role_outputs.len() == 1 {
        role_outputs
            .first()
            .map(|(_, output)| output.clone())
            .unwrap_or_default()
    } else {
        role_outputs
            .iter()
            .map(|(role, output)| format!("[{}]\n{}", role, output))
            .collect::<Vec<_>>()
            .join("\n\n")
    };

    Ok(AssistantChatResponse {
        ok: !any_acp_error,
        reply,
        runtime_kind: Some(assistant),
        session_id: None,
        command_result: None,
    })
}
