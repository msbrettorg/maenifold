// T-303.4.19 blue-team contract tests.
//
// Research note: Context7 lookup was attempted first but the service returned a
// monthly-quota error. These source-level assertions use Rust's documented
// `include_str!` compile-time file inclusion pattern from
// https://doc.rust-lang.org/std/macro.include_str.html.

fn execute_runtime_sampling_source() -> &'static str {
    include_str!("../../maenifold-jockey/src/acp/session/execute.rs")
}

#[tokio::test]
async fn execute_runtime_sampling_accepts_selected_model_some_contract() {
    let app_session_id = uuid::Uuid::new_v4();
    let expected_model = "gpt-5.5".to_string();

    let _ = jockey::acp::execute_runtime_sampling(
        "mock",
        "copilot",
        app_session_id,
        "Summarize this context".to_string(),
        Some(expected_model),
    )
    .await;
}

#[tokio::test]
async fn execute_runtime_sampling_accepts_selected_model_none_contract() {
    let app_session_id = uuid::Uuid::new_v4();

    let _ = jockey::acp::execute_runtime_sampling(
        "mock",
        "copilot",
        app_session_id,
        "Summarize this context".to_string(),
        None,
    )
    .await;
}

#[test]
fn execute_runtime_sampling_threads_selected_model_into_role_config_options() {
    let source = execute_runtime_sampling_source();
    let function_start = source
        .find("pub async fn execute_runtime_sampling")
        .expect("execute_runtime_sampling must exist");
    let function = &source[function_start..];

    assert!(
        function.contains("selected_model: Option<String>"),
        "T-303.4.19: execute_runtime_sampling must accept selected_model: Option<String>"
    );
    assert!(
        function.contains("\"model\".to_string()") && function.contains("selected_model"),
        "T-303.4.19: selected_model=Some(X) must become role_config_options entry (\"model\", X)"
    );
    assert!(
        !function.contains("true, vec![], None, vec![], None"),
        "T-303.4.19: execute_runtime_sampling must not hard-code empty role_config_options"
    );
}
