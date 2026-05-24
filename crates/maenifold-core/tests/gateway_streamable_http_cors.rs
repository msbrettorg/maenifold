mod common;

use common::gateway_http::{
    TestServer, broken_server, gateway_config, initialize, rpc, tool_names,
};
use reqwest::header::ORIGIN;
use serde_json::Value;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn streamable_http_cors_allows_only_local_file_and_null_origins() {
    let server = TestServer::start(gateway_config(vec![])).await;
    for origin in [
        "null",
        "file://",
        "http://localhost:8080",
        "http://127.0.0.1:8080",
        "http://[::1]:8080",
    ] {
        let response = server
            .client
            .request(reqwest::Method::OPTIONS, &server.endpoint)
            .header(ORIGIN, origin)
            .header("access-control-request-method", "POST")
            .send()
            .await
            .expect("CORS preflight request");
        assert_eq!(
            response
                .headers()
                .get("access-control-allow-origin")
                .and_then(|value| value.to_str().ok()),
            Some(origin),
            "allowed origin should be echoed: {origin}"
        );
    }

    let denied = server
        .client
        .request(reqwest::Method::OPTIONS, &server.endpoint)
        .header(ORIGIN, "https://example.com")
        .header("access-control-request-method", "POST")
        .send()
        .await
        .expect("denied CORS preflight request");
    assert!(
        denied
            .headers()
            .get("access-control-allow-origin")
            .is_none(),
        "non-local origin must not receive Access-Control-Allow-Origin"
    );

    server.stop().await;
}

#[tokio::test]
#[serial]
async fn failed_gateway_startup_does_not_return_native_only_healthy_tool_list() {
    let server = TestServer::start(gateway_config(vec![
        broken_server("broken-a"),
        broken_server("broken-b"),
    ]))
    .await;
    let init = initialize(&server).await;
    assert!(init.get("result").is_some());

    let list = rpc(&server, "tools/list", Value::Null, None).await;
    assert!(
        list.get("error").is_none(),
        "degraded startup should keep tools/list available: {list:?}"
    );

    let mut names = tool_names(&list);
    names.sort();
    assert_eq!(
        names,
        vec![
            "GatewayBatch",
            "GatewayExecute",
            "GatewayIndex",
            "GatewayIntent",
            "GetHelp",
            "ReadMcpResource",
            "complete_node",
        ],
        "degraded startup must preserve the public seven-tool surface"
    );

    server.stop().await;
}
