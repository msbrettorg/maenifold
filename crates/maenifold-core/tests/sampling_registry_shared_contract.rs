// T-303.4.14 blue-team contract tests.
//
// Research note: Context7 lookup was attempted first but the service returned a
// monthly-quota error. These source-level tests use Rust's documented
// `include_str!` compile-time file inclusion pattern from
// https://doc.rust-lang.org/std/macro.include_str.html.

fn client_handler_source() -> &'static str {
    include_str!("../src/mcp/client_handler.rs")
}

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

#[test]
fn client_constructor_does_not_set_global_sampling_registry() {
    let source = client_handler_source();
    let constructor_start = source
        .find("fn new_with_sampling_registry")
        .expect("MaenifoldClient::new_with_sampling_registry must exist");
    let constructor_end = source[constructor_start..]
        .find("impl ClientHandler for MaenifoldClient")
        .expect("client handler impl follows constructor")
        + constructor_start;
    let constructor = &source[constructor_start..constructor_end];
    let calls = constructor.matches("set_registry").count();

    assert_eq!(
        calls, 0,
        "T-303.4.14: MaenifoldClient::new_with_sampling_registry must not call set_registry; global sampling registry ownership belongs to build_gateway_with_registry"
    );
}

#[test]
fn build_gateway_with_registry_sets_sampling_registry_exactly_once() {
    let source = gateway_startup_source();
    let calls = source.matches("set_registry").count();

    assert_eq!(
        calls, 1,
        "T-303.4.14: build_gateway_with_registry must call resources::sampling::set_registry exactly once before constructing any StdioPool branch"
    );
}

#[test]
fn stdio_pool_carries_shared_sampling_registry_across_sessions() {
    let pool = stdio_pool_source();
    let session = stdio_session_source();

    assert!(
        pool.contains("SamplingSessionRegistry"),
        "T-303.4.14: StdioPool must import/use SamplingSessionRegistry so all stdio clients share one Arc"
    );
    assert!(
        pool.contains("sampling_registry: Arc<SamplingSessionRegistry>")
            || pool.contains("sampling_session_registry: Arc<SamplingSessionRegistry>"),
        "T-303.4.14: StdioPool must store Arc<SamplingSessionRegistry> for initial start and restart paths"
    );
    assert!(
        !session
            .contains("Arc::new(super::sampling_session_registry::SamplingSessionRegistry::new())"),
        "T-303.4.14: StdioSession must not allocate a fresh SamplingSessionRegistry per client; pass the shared Arc through"
    );
}

#[test]
fn restart_server_reuses_pool_sampling_registry() {
    let lifecycle = stdio_pool_lifecycle_source();

    assert!(
        !lifecycle.contains("start_session(&config)"),
        "T-303.4.14: StdioPool::restart_server must not call start_session(&config), because that path creates a fresh sampling registry"
    );
    assert!(
        lifecycle.contains("sampling_registry")
            || lifecycle.contains("start_session_with_registry(&config"),
        "T-303.4.14: restart_server must pass the pool-owned Arc<SamplingSessionRegistry> into the replacement session"
    );
}
