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
    {
        if let Some(updated_config_options) =
            set_config_option(conn, session_id, config_id, model.to_string()).await
        {
            config_options =
                preserve_config_option_categories(&config_options, updated_config_options);
        }
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
    use std::{
        collections::VecDeque,
        path::PathBuf,
        sync::{Arc, Mutex},
    };

    use sacp::{Agent, Client};
    use tokio::task::LocalSet;
    use tokio_util::compat::{TokioAsyncReadCompatExt as _, TokioAsyncWriteCompatExt as _};

    use super::*;

    fn session_opts(model: Option<&str>, thought_level: Option<&str>) -> SessionOpts {
        SessionOpts {
            cwd: PathBuf::from("."),
            auto_grant_permissions: true,
            model: model.map(str::to_string),
            mode: None,
            thought_level: thought_level.map(str::to_string),
            config_options: None,
            allowed_tools: None,
            max_turns: None,
            timeout_seconds: None,
            mcp_servers: Vec::new(),
        }
    }

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

    async fn run_modern_config_flow(
        initial_config_options: Vec<acp::SessionConfigOption>,
        opts: SessionOpts,
        response_plan: Vec<Vec<acp::SessionConfigOption>>,
    ) -> Vec<(String, String)> {
        run_config_flow(initial_config_options, opts, true, response_plan).await
    }

    async fn run_legacy_config_flow(
        opts: SessionOpts,
        response_plan: Vec<Vec<acp::SessionConfigOption>>,
    ) -> Vec<(String, String)> {
        run_config_flow(Vec::new(), opts, false, response_plan).await
    }

    async fn run_config_flow(
        initial_config_options: Vec<acp::SessionConfigOption>,
        opts: SessionOpts,
        is_modern: bool,
        response_plan: Vec<Vec<acp::SessionConfigOption>>,
    ) -> Vec<(String, String)> {
        let local = LocalSet::new();

        local
            .run_until(async move {
                let requests = Arc::new(Mutex::new(Vec::<(String, String)>::new()));
                let responses = Arc::new(Mutex::new(VecDeque::from(response_plan)));

                let (client_writer, server_reader) = tokio::io::duplex(4096);
                let (server_writer, client_reader) = tokio::io::duplex(4096);

                let server_transport =
                    sacp::ByteStreams::new(server_writer.compat_write(), server_reader.compat());
                let client_transport =
                    sacp::ByteStreams::new(client_writer.compat_write(), client_reader.compat());

                let server_requests = Arc::clone(&requests);
                let server_responses = Arc::clone(&responses);
                tokio::task::spawn_local(async move {
                    let server = Agent.builder().on_receive_request(
                        async move |request: sacps::SetSessionConfigOptionRequest,
                                    responder: sacp::Responder<
                            sacps::SetSessionConfigOptionResponse,
                        >,
                                    _connection: sacp::ConnectionTo<sacp::Client>| {
                            server_requests.lock().expect("requests mutex poisoned").push((
                                request.config_id.to_string(),
                                request.value.to_string(),
                            ));

                            let config_options = server_responses
                                .lock()
                                .expect("responses mutex poisoned")
                                .pop_front()
                                .expect("unexpected set_config_option request");
                            let response: sacps::SetSessionConfigOptionResponse =
                                bridge(acp::SetSessionConfigOptionResponse::new(config_options))
                                    .expect("bridge fake response");
                            responder.respond(response)
                        },
                        sacp::on_receive_request!(),
                    );

                    if let Err(error) = server.connect_to(server_transport).await {
                        panic!("fake ACP agent failed: {error}");
                    }
                });

                let result = Client
                    .builder()
                    .connect_with(client_transport, async move |conn| {
                        apply_session_config(
                            &conn,
                            "session-1",
                            &opts,
                            is_modern,
                            &initial_config_options,
                        )
                        .await;
                        Ok(())
                    })
                    .await;

                assert!(result.is_ok(), "fake ACP client failed: {result:?}");
                assert_eq!(
                    responses.lock().expect("responses mutex poisoned").len(),
                    0,
                    "unused fake responses usually means chained config stopped early"
                );

                requests.lock().expect("requests mutex poisoned").clone()
            })
            .await
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

    #[tokio::test(flavor = "current_thread")]
    async fn codex_chained_config_flow_still_sets_reasoning_effort_after_model_update() {
        let requests = run_modern_config_flow(
            vec![
                model_option("gpt-4"),
                reasoning_effort_option(
                    "high",
                    Some(acp::SessionConfigOptionCategory::ThoughtLevel),
                ),
            ],
            session_opts(Some("gpt-5-codex"), Some("low")),
            vec![
                vec![
                    model_option("gpt-5-codex"),
                    reasoning_effort_option("high", None),
                ],
                vec![
                    model_option("gpt-5-codex"),
                    reasoning_effort_option("low", None),
                ],
            ],
        )
        .await;

        assert_eq!(
            requests,
            vec![
                ("model".to_string(), "gpt-5-codex".to_string()),
                ("reasoning_effort".to_string(), "low".to_string()),
            ]
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn agents_without_thought_level_or_reasoning_effort_do_not_receive_extra_request() {
        let requests = run_modern_config_flow(
            vec![model_option("gpt-4")],
            session_opts(Some("gpt-5-codex"), Some("low")),
            vec![vec![model_option("gpt-5-codex")]],
        )
        .await;

        assert_eq!(
            requests,
            vec![("model".to_string(), "gpt-5-codex".to_string())]
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn legacy_config_flow_keeps_using_literal_thought_level_request() {
        let requests = run_legacy_config_flow(session_opts(None, Some("low")), vec![vec![]]).await;

        assert_eq!(
            requests,
            vec![("thought_level".to_string(), "low".to_string())]
        );
    }
}
