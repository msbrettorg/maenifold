use agent_client_protocol::{self as acp, Agent as _};
use super::types::SessionOpts;
use tracing::warn;

pub(super) async fn apply_session_config(
    conn: &acp::ClientSideConnection,
    session_id: acp::SessionId,
    opts: &SessionOpts,
    is_modern: bool,
    config_options: &[acp::SessionConfigOption],
) {
    if is_modern {
        apply_modern_config(conn, session_id, opts, config_options).await;
    } else {
        apply_legacy_config(conn, session_id.clone(), opts).await;
        apply_extra_config_options(conn, session_id, opts).await;
    }
}

async fn apply_modern_config(
    conn: &acp::ClientSideConnection,
    session_id: acp::SessionId,
    opts: &SessionOpts,
    config_options: &[acp::SessionConfigOption],
) {
    if let Some(model) = opts.model.as_deref() {
        if let Some(config_id) = find_config_id_by_category(config_options, "model") {
            set_config_option(conn, session_id.clone(), config_id, model.to_string()).await;
        }
    }
    if let Some(mode) = opts.mode.as_deref() {
        if let Some(config_id) = find_config_id_by_category(config_options, "mode") {
            set_config_option(conn, session_id.clone(), config_id, mode.to_string()).await;
        } else {
            // mode is a standard ACP method — safe to call even for modern agents
            apply_optional(Some(mode.to_string()), "ACP session mode", |mode| {
                conn.set_session_mode(acp::SetSessionModeRequest::new(session_id.clone(), mode))
            })
            .await;
        }
    }
    if let Some(thought_level) = opts.thought_level.as_deref() {
        if let Some(config_id) = find_config_id_by_category(config_options, "thought_level") {
            set_config_option(
                conn,
                session_id.clone(),
                config_id,
                thought_level.to_string(),
            )
            .await;
        } else {
            set_config_option(
                conn,
                session_id.clone(),
                "thought_level".to_string(),
                thought_level.to_string(),
            )
            .await;
        }
    }
    apply_extra_config_options(conn, session_id, opts).await;
}

async fn apply_legacy_config(
    conn: &acp::ClientSideConnection,
    session_id: acp::SessionId,
    opts: &SessionOpts,
) {
    apply_optional(opts.model.clone(), "ACP session model", |model| {
        conn.set_session_model(acp::SetSessionModelRequest::new(session_id.clone(), model))
    })
    .await;

    apply_optional(opts.mode.clone(), "ACP session mode", |mode| {
        conn.set_session_mode(acp::SetSessionModeRequest::new(session_id.clone(), mode))
    })
    .await;

    if let Some(thought_level) = opts.thought_level.clone() {
        set_config_option(conn, session_id, "thought_level".to_string(), thought_level).await;
    }
}

async fn apply_extra_config_options(
    conn: &acp::ClientSideConnection,
    session_id: acp::SessionId,
    opts: &SessionOpts,
) {
    if let Some(config_options) = opts.config_options.clone() {
        for (config_id, value) in config_options {
            if matches!(
                config_id.as_str(),
                "thought_level" | "allowed_tools" | "max_turns"
            ) {
                continue;
            }
            set_config_option(
                conn,
                session_id.clone(),
                config_id,
                config_option_value(value),
            )
            .await;
        }
    }

    if let Some(allowed_tools) = opts.allowed_tools.clone() {
        set_config_option(
            conn,
            session_id.clone(),
            "allowed_tools".to_string(),
            serde_json::to_string(&allowed_tools).unwrap_or_else(|_| "[]".to_string()),
        )
        .await;
    }

    if let Some(max_turns) = opts.max_turns {
        set_config_option(
            conn,
            session_id,
            "max_turns".to_string(),
            max_turns.to_string(),
        )
        .await;
    }
}

fn find_config_id_by_category(
    config_options: &[acp::SessionConfigOption],
    category_name: &str,
) -> Option<String> {
    config_options.iter().find_map(|opt| {
        let matches = match (&opt.category, category_name) {
            (Some(acp::SessionConfigOptionCategory::Model), "model") => true,
            (Some(acp::SessionConfigOptionCategory::Mode), "mode") => true,
            (Some(acp::SessionConfigOptionCategory::ThoughtLevel), "thought_level") => true,
            (Some(acp::SessionConfigOptionCategory::Other(value)), name) => value == name,
            _ => false,
        };
        matches.then(|| opt.id.to_string())
    })
}

fn config_option_value(value: serde_json::Value) -> String {
    match value {
        serde_json::Value::String(value) => value,
        value => value.to_string(),
    }
}

async fn apply_optional<T, F, Fut, R>(value: Option<T>, description: &str, apply: F)
where
    F: FnOnce(T) -> Fut,
    Fut: std::future::Future<Output = acp::Result<R>>,
{
    if let Some(value) = value
        && let Err(error) = apply(value).await
    {
        eprintln!("[ACP] Failed to set {description}: {error}");
    }
}

async fn set_config_option(
    conn: &acp::ClientSideConnection,
    session_id: acp::SessionId,
    config_id: String,
    value: String,
) {
    eprintln!("[ACP] set_config_option: {config_id}={value}");
    if let Err(error) = conn
        .set_session_config_option(acp::SetSessionConfigOptionRequest::new(
            session_id,
            config_id.clone(),
            value,
        ))
        .await
    {
        eprintln!("[ACP] Failed to set ACP session config {config_id}: {error}");
    }
}
