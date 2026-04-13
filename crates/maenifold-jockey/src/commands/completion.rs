use crate::db::context::{list_all_known_models, list_dynamic_catalog};
use crate::db::role::list_all_roles;
use crate::fs_context::{
    is_within_workspace, relative_or_abs, resolve_attach_path, should_skip_name,
};
use crate::resolve_chat_cwd;
use crate::types::*;
use serde::Serialize;
use std::collections::HashSet;
use std::sync::OnceLock;

#[derive(Copy, Clone, Eq, PartialEq)]
enum MentionPathMode {
    Auto,
    File,
    Dir,
}

fn normalize_slashes(input: &str) -> String {
    input.replace('\\', "/")
}

fn parse_mention_query(query: &str) -> (MentionPathMode, String) {
    let trimmed = query.trim();
    if let Some(rest) = trimmed.strip_prefix("file:") {
        return (MentionPathMode::File, rest.trim().to_string());
    }
    if let Some(rest) = trimmed.strip_prefix("dir:") {
        return (MentionPathMode::Dir, rest.trim().to_string());
    }
    (MentionPathMode::Auto, trimmed.to_string())
}

fn split_parent_and_prefix(raw: &str) -> (String, String) {
    let normalized = normalize_slashes(raw);
    if let Some(parent) = normalized.strip_suffix("/**") {
        if parent.is_empty() {
            return (".".to_string(), "".to_string());
        }
        return (parent.to_string(), "".to_string());
    }
    if normalized.is_empty() {
        return (".".to_string(), "".to_string());
    }
    if normalized.ends_with('/') {
        let parent = normalized.trim_end_matches('/');
        if parent.is_empty() {
            return ("/".to_string(), "".to_string());
        }
        return (parent.to_string(), "".to_string());
    }
    if let Some(idx) = normalized.rfind('/') {
        let parent = &normalized[..idx];
        let prefix = &normalized[idx + 1..];
        if parent.is_empty() {
            return ("/".to_string(), prefix.to_string());
        }
        return (parent.to_string(), prefix.to_string());
    }
    (".".to_string(), normalized)
}

fn complete_path_mentions(cwd: &str, query: &str, limit: usize) -> Vec<MentionCandidate> {
    let (mode, path_query) = parse_mention_query(query);
    let (parent_raw, name_prefix) = split_parent_and_prefix(&path_query);
    let parent_abs = resolve_attach_path(cwd, &parent_raw);
    if !is_within_workspace(&parent_abs, cwd) {
        return Vec::new();
    }
    if !parent_abs.exists() || !parent_abs.is_dir() {
        return Vec::new();
    }
    let name_prefix_lower = name_prefix.to_ascii_lowercase();
    let parent_normalized = normalize_slashes(&parent_raw);
    let mut rows: Vec<(bool, String)> = Vec::new();

    let Ok(entries) = std::fs::read_dir(&parent_abs) else {
        return Vec::new();
    };
    for entry in entries.flatten() {
        let file_name = entry.file_name().to_string_lossy().to_string();
        if should_skip_name(&file_name) {
            continue;
        }
        if !name_prefix_lower.is_empty()
            && !file_name
                .to_ascii_lowercase()
                .starts_with(&name_prefix_lower)
        {
            continue;
        }
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        let is_dir = ft.is_dir();
        if mode == MentionPathMode::Dir && !is_dir {
            continue;
        }
        if mode == MentionPathMode::File && !ft.is_file() {
            continue;
        }
        let mut rel = if parent_normalized == "." {
            file_name.clone()
        } else if parent_normalized == "/" {
            format!("/{file_name}")
        } else {
            format!("{parent_normalized}/{file_name}")
        };
        rel = normalize_slashes(&rel);
        if is_dir {
            rel.push('/');
        }
        rows.push((is_dir, rel));
    }

    rows.sort_by(|a, b| match (a.0, b.0) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.1.cmp(&b.1),
    });

    rows.into_iter()
        .take(limit)
        .map(|(is_dir, rel)| {
            let (kind, value) = match mode {
                MentionPathMode::Dir => ("dir".to_string(), format!("dir:{rel}")),
                MentionPathMode::File => ("file".to_string(), format!("file:{rel}")),
                MentionPathMode::Auto => {
                    if is_dir {
                        ("dir".to_string(), rel)
                    } else {
                        ("file".to_string(), rel)
                    }
                }
            };
            MentionCandidate {
                value,
                kind,
                detail: relative_or_abs(&parent_abs, cwd),
            }
        })
        .collect()
}

pub(crate) async fn complete_mentions(
    _state: &AppState,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<MentionCandidate>, String> {
    let cwd = resolve_chat_cwd();
    let capped = limit.unwrap_or(10).clamp(1, 30);
    tokio::task::spawn_blocking(move || complete_path_mentions(&cwd, &query, capped))
        .await
        .map_err(|e| e.to_string())
}

fn matches_cli_query(candidate: &str, query_lower: &str) -> bool {
    if query_lower.is_empty() {
        return true;
    }
    let candidate_lower = candidate.to_ascii_lowercase();
    candidate_lower.starts_with(query_lower) || candidate_lower.contains(query_lower)
}

fn push_cli_candidate(
    out: &mut Vec<MentionCandidate>,
    seen: &mut HashSet<String>,
    value: String,
    detail: String,
) {
    if seen.insert(value.clone()) {
        out.push(MentionCandidate {
            value,
            kind: "command".to_string(),
            detail,
        });
    }
}

fn push_matching_cli_template(
    out: &mut Vec<MentionCandidate>,
    seen: &mut HashSet<String>,
    query_lower: &str,
    value: &str,
    detail: &str,
) {
    if !matches_cli_query(value, query_lower) {
        return;
    }
    push_cli_candidate(out, seen, value.to_string(), detail.to_string());
}

fn append_static_cli_templates(
    out: &mut Vec<MentionCandidate>,
    seen: &mut HashSet<String>,
    query_lower: &str,
) {
    for (value, detail) in BASE_CLI_COMMANDS {
        push_matching_cli_template(out, seen, query_lower, value, detail);
    }
    for (value, detail) in acp_protocol_cli_templates() {
        let detail = format!("{detail} (template)");
        push_matching_cli_template(out, seen, query_lower, value.as_str(), &detail);
    }
}

fn append_dynamic_catalog_cli_templates(
    state: &AppState,
    out: &mut Vec<MentionCandidate>,
    seen: &mut HashSet<String>,
    query_lower: &str,
) {
    let models = list_all_known_models(state);
    for model in &models {
        let value = format!("/app_model select {model}");
        push_matching_cli_template(out, seen, query_lower, &value, "Select configured model");
    }

    let mcps = list_dynamic_catalog(state, "mcp").unwrap_or_default();
    for mcp in &mcps {
        for value in [
            format!("/app_mcp enable {mcp}"),
            format!("/app_mcp disable {mcp}"),
            format!("/app_mcp remove {mcp}"),
        ] {
            push_matching_cli_template(out, seen, query_lower, &value, "MCP server command");
        }
    }
}

fn append_role_cli_templates(
    state: &AppState,
    out: &mut Vec<MentionCandidate>,
    seen: &mut HashSet<String>,
    query_lower: &str,
) -> Result<(), String> {
    let roles = list_all_roles(state).unwrap_or_default();
    for role in &roles {
        let role_name = role.role_name.clone();
        for value in [
            format!("/app_role prompt {} ", role_name),
            format!("/app_model select role {} <model>", role_name),
        ] {
            push_matching_cli_template(out, seen, query_lower, &value, "Role command");
        }
    }
    Ok(())
}

fn build_cli_completion_candidates(
    state: &AppState,
    query: &str,
    limit: usize,
) -> Result<Vec<MentionCandidate>, String> {
    let query_lower = query.trim().to_ascii_lowercase();
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    append_static_cli_templates(&mut out, &mut seen, &query_lower);
    append_dynamic_catalog_cli_templates(state, &mut out, &mut seen, &query_lower);
    append_role_cli_templates(state, &mut out, &mut seen, &query_lower)?;

    if out.len() > limit {
        out.truncate(limit);
    }
    Ok(out)
}

pub(crate) fn complete_cli(
    state: &AppState,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<MentionCandidate>, String> {
    let capped = limit.unwrap_or(20).clamp(1, 60);
    build_cli_completion_candidates(state, &query, capped)
}

pub(crate) fn push_acp_cli_template(
    out: &mut Vec<(String, String)>,
    method: &str,
    args_template: &str,
    detail: &str,
) {
    let value = if args_template.is_empty() {
        format!("/acp {method}")
    } else {
        format!("/acp {method} {args_template}")
    };
    out.push((value, detail.to_string()));
}

pub(crate) fn collect_acp_method_names<T: Serialize>(methods: &T) -> Vec<String> {
    let Ok(value) = serde_json::to_value(methods) else {
        return Vec::new();
    };
    let Some(map) = value.as_object() else {
        return Vec::new();
    };
    map.values()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect()
}

pub(crate) fn acp_method_args_template(method: &str) -> &'static str {
    match method {
        "initialize" => "",
        "authenticate" => "<token>",
        "session/new" => "<cwd>",
        "session/load" => "<sessionId>",
        "session/list" => "",
        "session/fork" => "<sessionId>",
        "session/resume" => "<sessionId>",
        "session/close" => "<sessionId>",
        "session/prompt" => "<text>",
        "session/cancel" => "<requestId>",
        "session/set_mode" => "<mode>",
        "session/set_model" => "<model>",
        "session/set_config_option" => "<key> <value>",
        "session/request_permission" => "<permission>",
        "session/update" => "<event>",
        "fs/read_text_file" => "<path>",
        "fs/write_text_file" => "<path> <content>",
        "terminal/create" => "<cwd> [shell]",
        "terminal/output" => "<terminalId>",
        "terminal/wait_for_exit" => "<terminalId>",
        "terminal/release" => "<terminalId>",
        "terminal/kill" => "<terminalId>",
        "$/cancel_request" => "<requestId>",
        _ => "",
    }
}

pub(crate) fn acp_method_detail(method: &str) -> &'static str {
    match method {
        "initialize" => "ACP request: protocol initialize",
        "authenticate" => "ACP request: authenticate",
        "session/new" => "ACP request: create session",
        "session/load" => "ACP request: load session",
        "session/list" => "ACP request: list sessions",
        "session/fork" => "ACP request: fork session (unstable)",
        "session/resume" => "ACP request: resume session (unstable)",
        "session/close" => "ACP request: close session (unstable)",
        "session/prompt" => "ACP request: prompt",
        "session/cancel" => "ACP notification: cancel in-flight request",
        "session/set_mode" => "ACP request: update session mode",
        "session/set_model" => "ACP request: set session model (unstable)",
        "session/set_config_option" => "ACP request: set session config option",
        "session/request_permission" => "ACP request: ask client for permission",
        "session/update" => "ACP notification: session update",
        "fs/read_text_file" => "ACP request: read text file",
        "fs/write_text_file" => "ACP request: write text file",
        "terminal/create" => "ACP request: create terminal",
        "terminal/output" => "ACP notification: terminal output stream",
        "terminal/wait_for_exit" => "ACP request: wait terminal exit",
        "terminal/release" => "ACP request: release terminal handle",
        "terminal/kill" => "ACP request: kill terminal",
        "$/cancel_request" => "ACP protocol notification: cancel request (unstable)",
        _ => "ACP method from SDK",
    }
}

pub(crate) fn acp_protocol_cli_templates() -> &'static Vec<(String, String)> {
    static TEMPLATES: OnceLock<Vec<(String, String)>> = OnceLock::new();
    TEMPLATES.get_or_init(|| {
        let mut out = Vec::new();
        let mut method_names = collect_acp_method_names(&agent_client_protocol::AGENT_METHOD_NAMES);
        method_names.extend(collect_acp_method_names(
            &agent_client_protocol::CLIENT_METHOD_NAMES,
        ));
        method_names.sort_unstable();
        method_names.dedup();

        for method in method_names {
            push_acp_cli_template(
                &mut out,
                &method,
                acp_method_args_template(&method),
                acp_method_detail(&method),
            );
        }

        out.push((
            "/acp extMethod <name> <jsonPayload>".to_string(),
            "ACP request: extension method".to_string(),
        ));
        out.push((
            "/acp extNotification <name> <jsonPayload>".to_string(),
            "ACP notification: extension event".to_string(),
        ));
        out
    })
}
