use super::*;

#[tokio::test]
#[serial_test::serial]
async fn invoke_external_tool_preserves_http_auth_and_custom_headers() {
    let fake_cli_dir = TempDir::new().expect("fake Azure CLI dir");
    install_fake_azure_cli(fake_cli_dir.path(), "TOKEN_4242");
    let _path_guard = prepend_path_guard(fake_cli_dir.path());

    let state = HeaderCaptureState {
        requests: Arc::new(Mutex::new(Vec::new())),
    };
    let app = Router::new()
        .route(
            "/.well-known/oauth-protected-resource/",
            get(oauth_metadata_handler),
        )
        .route(
            "/mcp",
            post(capture_mcp_headers_handler).delete(delete_mcp_session_handler),
        )
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind test HTTP server");
    let port = listener.local_addr().expect("listener addr").port();
    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("serve test HTTP server");
    });

    let config = config_with_global_and_agents(
        HashMap::from([(
            "remote".to_string(),
            McpServerEntry::Http {
                url: format!("http://127.0.0.1:{port}/mcp"),
                headers: HashMap::from([(
                    "X-Custom-Header".to_string(),
                    "marker-value-9876".to_string(),
                )]),
                auth: Some(HttpAuthConfig::AzureDefault),
                timeout: None,
                gateway: false,
            },
        )]),
        [],
    );
    let context = ToolContext::new(Config::current(), OutputFormat::Json);

    let result = invoke_external_tool(
        &context,
        &config,
        "remote",
        "echo",
        Some(&json!({ "value": "ignored" })),
        None,
        None,
        Path::new("."),
    )
    .await;

    server_handle.abort();

    let output = result.expect("external MCP HTTP tool call should succeed");
    assert_eq!(output, "ok");

    let requests = state.requests.lock().await;
    let tool_call_headers = requests
        .iter()
        .find(|(method, _)| method == "tools/call")
        .map(|(_, headers)| headers)
        .expect("captured tools/call request");

    assert_eq!(
        tool_call_headers.get("x-custom-header"),
        Some(&"marker-value-9876".to_string()),
        "HTTP MCP tools/call request should include custom headers from McpServerEntry::Http"
    );
    assert_eq!(
        tool_call_headers.get("authorization"),
        Some(&"Bearer TOKEN_4242".to_string()),
        "HTTP MCP tools/call request should include Authorization derived from McpServerEntry::Http auth"
    );
}
