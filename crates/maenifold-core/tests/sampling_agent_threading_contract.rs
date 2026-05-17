// T-303.4.15 blue-team contract tests.
//
// Research note: Context7 lookup was attempted first but the service returned a
// monthly-quota error. These source-level tests use Rust's documented
// `include_str!` compile-time file inclusion pattern from
// https://doc.rust-lang.org/std/macro.include_str.html.

fn gateway_startup_source() -> &'static str {
    include_str!("../src/mcp/gateway_startup.rs")
}

fn stdio_pool_source() -> &'static str {
    include_str!("../src/mcp/stdio_pool.rs")
}

fn stdio_pool_lifecycle_source() -> &'static str {
    include_str!("../src/mcp/stdio_pool_lifecycle.rs")
}

fn stdio_session_source() -> &'static str {
    include_str!("../src/mcp/stdio_session.rs")
}

fn client_handler_tests_source() -> &'static str {
    include_str!("../src/mcp/client_handler_tests.rs")
}

#[test]
fn gateway_threads_configured_sampling_agent_into_stdio_pool() {
    let source = gateway_startup_source();

    assert!(
        source.contains("agents_config.sampling_agent.clone()")
            || source.contains("agents_config.sampling_agent.as_ref().cloned()"),
        "T-303.4.15: build_gateway_with_registry must read AgentsConfig.sampling_agent from maenifold.json"
    );
    assert!(
        source.contains("StdioPool::start_with_registry(configs, registry, sampling_agent")
            || source.contains("StdioPool::start_with_registry(configs, registry, agents_config.sampling_agent.clone()")
            || source.contains("StdioPool::start_with_sampling"),
        "T-303.4.15: build_gateway_with_registry must pass samplingAgent into StdioPool for every stdio server"
    );
}

#[test]
fn stdio_session_uses_threaded_sampling_agent_instead_of_none() {
    let source = stdio_session_source();

    assert!(
        !source.contains("let sampling_agent = None;"),
        "T-303.4.15: stdio_session.rs must not hardcode let sampling_agent = None; the configured Option<String> must reach MaenifoldClient"
    );
    assert!(
        source.contains("sampling_agent"),
        "T-303.4.15: StdioSession start path must accept and forward sampling_agent into MaenifoldClient::new_with_sampling_registry"
    );
}

#[test]
fn restart_server_reuses_pool_sampling_agent() {
    let pool = stdio_pool_source();
    let lifecycle = stdio_pool_lifecycle_source();

    assert!(
        pool.contains("sampling_agent: Option<String>"),
        "T-303.4.15: StdioPool must store sampling_agent: Option<String> so restarts inherit maenifold.json samplingAgent"
    );
    assert!(
        !lifecycle.contains("start_session(&config)"),
        "T-303.4.15: restart_server must not use the legacy start_session(&config) path that loses samplingAgent"
    );
    assert!(
        lifecycle.contains("sampling_agent"),
        "T-303.4.15: restart_server must pass the pool-owned sampling_agent into the replacement session"
    );
}

#[test]
fn no_inference_negative_32603_contract_remains_present() {
    // T-327.2: renamed from no_agent_*. The legacy samplingAgent-based test
    // (`create_message_without_sampling_agent_returns_negative_32603`) became
    // `create_message_without_inference_configured_returns_negative_32603`
    // when ACP/Jockey was removed from sampling fulfillment. -32603 still
    // fires, but the trigger is "direct inference not configured" instead of
    // "samplingAgent missing".
    let source = client_handler_tests_source();

    assert!(
        source.contains("create_message_without_inference_configured_returns_negative_32603"),
        "T-327.2: no-inference -32603 regression test must remain present"
    );
    assert!(
        source.contains("-32603"),
        "T-327.2: no-inference test must continue to assert JSON-RPC -32603 when neither sampling nor direct inference is configured"
    );
}
