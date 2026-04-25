mod common;

use common::gateway_http::{
    TestServer, error_tool, gateway_config, initialize, ok_tool, result_text, rpc, stdio_server,
    tool_names, ui_tool,
};
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::{Value, json};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn streamable_http_gateway_exercises_native_facade_passthrough_errors_and_resources() {
    let server = TestServer::start(gateway_config(vec![
        stdio_server(
            "big-server",
            vec![
                ok_tool("big_alpha", "handled alpha through facade"),
                ok_tool("big_beta", "handled beta through facade"),
                error_tool(
                    "big_fail",
                    -32042,
                    "upstream facade failure",
                    json!({"detail":"facade boom"}),
                ),
                ui_tool(
                    "big_dashboard",
                    "dashboard tool response",
                    "ui://big-server/dashboard",
                    "<section>big dashboard</section>",
                ),
            ],
        ),
        stdio_server(
            "small-server",
            vec![
                ok_tool("small_ping", "handled by passthrough"),
                error_tool(
                    "small_fail",
                    -32043,
                    "upstream passthrough failure",
                    json!({"detail":"passthrough boom"}),
                ),
            ],
        ),
    ]))
    .await;

    let init = initialize(&server).await;
    assert!(init["result"]["protocolVersion"].as_str().is_some());

    let list = rpc(&server, "tools/list", Value::Null, None).await;
    let names = tool_names(&list);
    assert!(names.iter().any(|name| name == "GetConfig"));
    assert!(names.iter().any(|name| name == "gateway_big_server"));
    assert!(names.iter().any(|name| name == "small_ping"));
    assert!(names.iter().any(|name| name == "small_fail"));
    assert!(!names.iter().any(|name| name == "big_alpha"));
    assert!(!names.iter().any(|name| name == "big_dashboard"));

    let native = rpc(
        &server,
        "tools/call",
        json!({"name":"GetConfig","arguments":{}}),
        None,
    )
    .await;
    assert!(result_text(&native).contains("\"maenifoldRoot\""));

    let facade = rpc(
        &server,
        "tools/call",
        json!({
            "name": "gateway_big_server",
            "arguments": {"command": "big_alpha", "parameters": {"message": "hello"}}
        }),
        None,
    )
    .await;
    assert_eq!(result_text(&facade), "handled alpha through facade");

    let hidden = rpc(
        &server,
        "tools/call",
        json!({"name":"big_alpha","arguments":{}}),
        None,
    )
    .await;
    assert_eq!(hidden["error"]["code"], -32602);
    assert!(
        hidden["error"]["message"]
            .as_str()
            .expect("hidden error message")
            .contains("gateway_big_server")
    );

    let passthrough = rpc(
        &server,
        "tools/call",
        json!({"name":"small_ping","arguments":{"message":"hello"}}),
        None,
    )
    .await;
    assert_eq!(result_text(&passthrough), "handled by passthrough");

    let resource = rpc(
        &server,
        "resources/read",
        json!({"uri":"ui://big-server/dashboard"}),
        None,
    )
    .await;
    assert_eq!(
        resource["result"]["contents"][0]["text"],
        "<section>big dashboard</section>"
    );

    let mut stale_headers = HeaderMap::new();
    stale_headers.insert(
        "mcp-session-id",
        HeaderValue::from_static("stale-or-unknown-session"),
    );
    let stale = rpc(&server, "tools/list", Value::Null, Some(stale_headers)).await;
    assert!(stale["result"]["tools"].as_array().is_some());

    let upstream_error = rpc(
        &server,
        "tools/call",
        json!({"name":"small_fail","arguments":{}}),
        None,
    )
    .await;
    assert_eq!(upstream_error["error"]["code"], -32043);
    assert_eq!(
        upstream_error["error"]["message"],
        "upstream passthrough failure"
    );
    assert_eq!(
        upstream_error["error"]["data"],
        json!({"detail":"passthrough boom"})
    );

    server.stop().await;
}
