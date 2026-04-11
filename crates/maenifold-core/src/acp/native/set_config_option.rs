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
    let mut config_options = config_options.to_vec();

    if let Some(model) = opts.model.as_deref()
        && let Some(config_id) = find_config_id_by_category(&config_options, "model")
        && let Some(updated_config_options) =
            set_config_option(conn, session_id, config_id, model.to_string()).await
    {
        config_options = preserve_config_option_categories(&config_options, updated_config_options);
    }
    if let Some(mode) = opts.mode.as_deref() {
        if let Some(config_id) = find_config_id_by_category(&config_options, "mode") {
            if let Some(updated_config_options) =
                set_config_option(conn, session_id, config_id, mode.to_string()).await
            {
                config_options =
                    preserve_config_option_categories(&config_options, updated_config_options);
            }
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
        if let Some(config_id) = find_thought_level_config_id(&config_options) {
            let _ = set_config_option(conn, session_id, config_id, thought_level.to_string()).await;
        } else {
            tracing::warn!(
                "Agent does not advertise thought_level config option; skipping (requested: {thought_level})"
            );
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
        let _ =
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
            let _ =
                set_config_option(conn, session_id, config_id, config_option_value(value)).await;
        }
    }

    if let Some(allowed_tools) = opts.allowed_tools.clone() {
        let _ = set_config_option(
            conn,
            session_id,
            "allowed_tools".to_string(),
            serde_json::to_string(&allowed_tools).unwrap_or_else(|_| "[]".to_string()),
        )
        .await;
    }

    if let Some(max_turns) = opts.max_turns {
        let _ = set_config_option(
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

fn find_thought_level_config_id(config_options: &[acp::SessionConfigOption]) -> Option<String> {
    find_config_id_by_category(config_options, "thought_level")
        .or_else(|| find_config_id_by_id(config_options, "thought_level"))
        .or_else(|| {
            config_options
                .iter()
                .find(|opt| opt.category.is_none() && opt.id.to_string() == "reasoning_effort")
                .map(|opt| opt.id.to_string())
        })
}

fn find_config_id_by_id(
    config_options: &[acp::SessionConfigOption],
    config_id: &str,
) -> Option<String> {
    config_options
        .iter()
        .find(|opt| opt.id.to_string() == config_id)
        .map(|opt| opt.id.to_string())
}

fn preserve_config_option_categories(
    previous: &[acp::SessionConfigOption],
    mut updated: Vec<acp::SessionConfigOption>,
) -> Vec<acp::SessionConfigOption> {
    for option in &mut updated {
        if option.category.is_none()
            && let Some(previous_category) = previous
                .iter()
                .find(|prior| prior.id == option.id)
                .and_then(|prior| prior.category.clone())
        {
            option.category = Some(previous_category);
        }
    }

    updated
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
) -> Option<Vec<acp::SessionConfigOption>> {
    let request = match build_set_config_request(session_id, config_id.clone(), value) {
        Ok(request) => request,
        Err(error) => {
            tracing::warn!("Failed to build ACP session config {config_id}: {error}");
            return None;
        }
    };

    match conn.send_request(request).block_task().await {
        Ok(response) => match bridge(response.config_options) {
            Ok(config_options) => Some(config_options),
            Err(error) => {
                tracing::warn!(
                    "Failed to decode ACP session config options after setting {config_id}: {error}"
                );
                None
            }
        },
        Err(error) => {
            tracing::warn!("Failed to set ACP session config {config_id}: {error}");
            None
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn select_option(
        id: &str,
        name: &str,
        current_value: &str,
        category: Option<acp::SessionConfigOptionCategory>,
    ) -> acp::SessionConfigOption {
        let option = acp::SessionConfigOption::select(
            id.to_string(),
            name.to_string(),
            current_value.to_string(),
            vec![acp::SessionConfigSelectOption::new(
                current_value.to_string(),
                current_value.to_string(),
            )],
        );

        match category {
            Some(category) => option.category(category),
            None => option,
        }
    }

    fn model_option(current_value: &str) -> acp::SessionConfigOption {
        select_option(
            "model",
            "Model",
            current_value,
            Some(acp::SessionConfigOptionCategory::Model),
        )
    }

    fn reasoning_effort_option(
        current_value: &str,
        category: Option<acp::SessionConfigOptionCategory>,
    ) -> acp::SessionConfigOption {
        select_option(
            "reasoning_effort",
            "Reasoning Effort",
            current_value,
            category,
        )
    }

    #[test]
    fn thought_level_lookup_prefers_explicit_thought_level_category() {
        let config_options = vec![select_option(
            "reasoning_effort",
            "Reasoning Effort",
            "high",
            Some(acp::SessionConfigOptionCategory::ThoughtLevel),
        )];

        assert_eq!(
            find_thought_level_config_id(&config_options),
            Some("reasoning_effort".to_string())
        );
    }

    #[test]
    fn thought_level_lookup_accepts_reasoning_effort_without_category() {
        let config_options = vec![reasoning_effort_option("high", None)];

        assert_eq!(
            find_thought_level_config_id(&config_options),
            Some("reasoning_effort".to_string())
        );
    }

    #[test]
    fn chained_model_update_response_still_resolves_reasoning_effort_when_category_drops() {
        let initial_config_options = vec![
            model_option("gpt-5-codex"),
            reasoning_effort_option("high", Some(acp::SessionConfigOptionCategory::ThoughtLevel)),
        ];
        let chained_response = vec![model_option("gpt-5"), reasoning_effort_option("high", None)];

        assert_eq!(
            find_thought_level_config_id(&initial_config_options),
            Some("reasoning_effort".to_string())
        );
        assert_eq!(
            find_thought_level_config_id(&chained_response),
            Some("reasoning_effort".to_string())
        );
    }

    #[test]
    fn thought_level_lookup_returns_none_when_agent_lacks_any_matching_option() {
        let config_options = vec![model_option("gpt-5-codex")];

        assert_eq!(find_thought_level_config_id(&config_options), None);
    }
}
