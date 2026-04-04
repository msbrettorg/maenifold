use agent_client_protocol as acp;
use sacp::{ConnectionTo, schema as sacps};

use super::{
    convert::bridge,
    types::{Result, SessionOpts},
};

pub(super) async fn apply_session_config(
    conn: &ConnectionTo<sacp::Agent>,
    session_id: impl Into<String>,
    opts: &SessionOpts,
    is_modern: bool,
    config_options: &[acp::SessionConfigOption],
) {
    let session_id = session_id.into();
    if is_modern {
        apply_modern_config(conn, &session_id, opts, config_options).await;
    } else {
        apply_legacy_config(conn, &session_id, opts).await;
        apply_extra_config_options(conn, &session_id, opts).await;
    }
}

async fn apply_modern_config(
    conn: &ConnectionTo<sacp::Agent>,
    session_id: &str,
    opts: &SessionOpts,
    config_options: &[acp::SessionConfigOption],
) {
    if let Some(model) = opts.model.as_deref()
        && let Some(config_id) = find_config_id_by_category(config_options, "model")
    {
        set_config_option(conn, session_id, config_id, model.to_string()).await;
    }
    if let Some(mode) = opts.mode.as_deref() {
        if let Some(config_id) = find_config_id_by_category(config_options, "mode") {
            set_config_option(conn, session_id, config_id, mode.to_string()).await;
        } else {
            apply_optional(Some(mode.to_string()), "ACP session mode", |mode| {
                conn.send_request(sacps::SetSessionModeRequest::new(
                    session_id.to_string(),
                    mode,
                ))
                .block_task()
            })
            .await;
        }
    }
    if let Some(thought_level) = opts.thought_level.as_deref() {
        if let Some(config_id) = find_config_id_by_category(config_options, "thought_level") {
            set_config_option(conn, session_id, config_id, thought_level.to_string()).await;
        } else {
            set_config_option(
                conn,
                session_id,
                "thought_level".to_string(),
                thought_level.to_string(),
            )
            .await;
        }
    }
    apply_extra_config_options(conn, session_id, opts).await;
}

async fn apply_legacy_config(
    conn: &ConnectionTo<sacp::Agent>,
    session_id: &str,
    opts: &SessionOpts,
) {
    apply_optional(opts.model.clone(), "ACP session model", |model| {
        conn.send_request(acp::ClientRequest::SetSessionModelRequest(
            acp::SetSessionModelRequest::new(acp::SessionId::new(session_id.to_string()), model),
        ))
        .block_task()
    })
    .await;

    apply_optional(opts.mode.clone(), "ACP session mode", |mode| {
        conn.send_request(sacps::SetSessionModeRequest::new(
            session_id.to_string(),
            mode,
        ))
        .block_task()
    })
    .await;

    if let Some(thought_level) = opts.thought_level.clone() {
        set_config_option(conn, session_id, "thought_level".to_string(), thought_level).await;
    }
}

async fn apply_extra_config_options(
    conn: &ConnectionTo<sacp::Agent>,
    session_id: &str,
    opts: &SessionOpts,
) {
    if let Some(config_options) = opts.config_options.clone() {
        for (config_id, value) in config_options {
            set_config_option(conn, session_id, config_id, config_option_value(value)).await;
        }
    }

    if let Some(allowed_tools) = opts.allowed_tools.clone() {
        set_config_option(
            conn,
            session_id,
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
    Fut: std::future::Future<Output = std::result::Result<R, sacp::Error>>,
{
    if let Some(value) = value
        && let Err(error) = apply(value).await
    {
        tracing::warn!("Failed to set {description}: {error}");
    }
}

async fn set_config_option(
    conn: &ConnectionTo<sacp::Agent>,
    session_id: &str,
    config_id: String,
    value: String,
) {
    let request = match build_set_config_request(session_id, config_id.clone(), value) {
        Ok(request) => request,
        Err(error) => {
            tracing::warn!("Failed to build ACP session config {config_id}: {error}");
            return;
        }
    };

    if let Err(error) = conn.send_request(request).block_task().await {
        tracing::warn!("Failed to set ACP session config {config_id}: {error}");
    }
}

fn build_set_config_request(
    session_id: &str,
    config_id: String,
    value: String,
) -> Result<sacps::SetSessionConfigOptionRequest> {
    let request = acp::SetSessionConfigOptionRequest::new(
        acp::SessionId::new(session_id.to_string()),
        config_id,
        value,
    );
    bridge(request)
}
