mod common;

use common::gateway_http::{
    TestServer, broken_server, error_tool, gateway_config, initialize, ok_tool, result_text, rpc,
    stdio_server, stdio_server_with_metadata, tool_names, ui_tool,
};
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::{Value, json};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn streamable_http_gateway_exercises_native_facade_passthrough_errors_and_resources() {
    let server = TestServer::start(gateway_config(vec![
        stdio_server_with_metadata(
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
            Some("Big upstream MCP server description."),
            Some("Use big-server for facade command coverage."),
        ),
        stdio_server_with_metadata(
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
            None,
            Some("Use small-server for passthrough command coverage."),
        ),
    ]))
    .await;

    let init = initialize(&server).await;
    assert!(init["result"]["protocolVersion"].as_str().is_some());
    let server_description = init["result"]["serverInfo"]["description"]
        .as_str()
        .expect("server description");
    let instructions = init["result"]["instructions"]
        .as_str()
        .expect("server instructions");
    assert!(server_description.contains("big-server"));
    assert!(server_description.contains("small-server"));
    assert!(server_description.contains("GatewayIndex"));
    assert!(instructions.contains("GatewayIndex with includeSchemas=true"));
    assert!(instructions.contains("ReadMcpResource only for concrete resource URIs"));

    let list = rpc(&server, "tools/list", Value::Null, None).await;
    let names = tool_names(&list);
    assert_tool_names(
        &names,
        &[
            "GatewayBatch",
            "GatewayExecute",
            "GatewayIndex",
            "GatewayIntent",
            "GetHelp",
            "ReadMcpResource",
            "complete_node",
        ],
    );
    assert!(!names.iter().any(|name| name == "GetConfig"));
    assert!(!names.iter().any(|name| name == "gateway_big_server"));
    assert!(!names.iter().any(|name| name == "small_ping"));
    assert!(!names.iter().any(|name| name == "small_fail"));
    assert!(!names.iter().any(|name| name == "big_alpha"));
    assert!(!names.iter().any(|name| name == "big_dashboard"));
    let tools = list["result"]["tools"].as_array().expect("tools array");
    assert_tool_description(
        tools,
        "GatewayIndex",
        "Start here on cold start. Lists the live gateway domains",
    );
    assert_tool_description(
        tools,
        "ReadMcpResource",
        "It does not discover gateway domains or read memory:// notes",
    );
    assert_tool_description(
        tools,
        "GetHelp",
        "it is not the first cold-start domain discovery call",
    );

    let domain_index = rpc(
        &server,
        "tools/call",
        json!({"name":"GatewayIndex","arguments":{}}),
        None,
    )
    .await;
    let domain_index_text = result_text(&domain_index);
    assert!(domain_index_text.contains("\"name\": \"big-server\""));
    assert!(domain_index_text.contains("Big upstream MCP server description."));
    assert!(domain_index_text.contains("Use big-server for facade command coverage."));
    assert!(domain_index_text.contains("\"name\": \"small-server\""));
    assert!(domain_index_text.contains("Use small-server for passthrough command coverage."));

    let config_index = rpc(
        &server,
        "tools/call",
        json!({
            "name":"GatewayIndex",
            "arguments":{"domain":"maenifold.system","includeSchemas":true}
        }),
        None,
    )
    .await;
    let config_index_text = result_text(&config_index);
    assert!(!config_index_text.contains("\"complete_node\""));
    assert!(config_index_text.contains("\"GetConfig\""));

    let native = rpc(
        &server,
        "tools/call",
        json!({
            "name":"GatewayExecute",
            "arguments":{"domain":"maenifold.system","command":"GetConfig","parameters":{}}
        }),
        None,
    )
    .await;
    assert!(result_text(&native).contains("\"maenifoldRoot\""));

    let mcp_resource = rpc(
        &server,
        "tools/call",
        json!({"name":"ReadMcpResource","arguments":{"uri":"asset://catalog"}}),
        None,
    )
    .await;
    assert!(!result_text(&mcp_resource).is_empty());

    let facade = rpc(
        &server,
        "tools/call",
        json!({
            "name": "GatewayExecute",
            "arguments": {"domain": "big-server", "command": "big_alpha", "parameters": {"message": "hello"}}
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
            .contains("GatewayExecute")
    );

    let passthrough = rpc(
        &server,
        "tools/call",
        json!({
            "name":"GatewayExecute",
            "arguments":{"domain":"small-server","command":"small_ping","parameters":{"message":"hello"}}
        }),
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
    let public_resource = rpc(
        &server,
        "tools/call",
        json!({"name":"ReadMcpResource","arguments":{"uri":"ui://big-server/dashboard"}}),
        None,
    )
    .await;
    assert!(
        result_text(&public_resource).contains("<section>big dashboard</section>"),
        "{}",
        result_text(&public_resource)
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
        json!({
            "name":"GatewayExecute",
            "arguments":{"domain":"small-server","command":"small_fail","parameters":{}}
        }),
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

#[tokio::test]
#[serial]
async fn streamable_http_gateway_startup_failure_keeps_public_surface_and_diagnostics() {
    let server = TestServer::start(gateway_config(vec![broken_server("broken-server")])).await;

    initialize(&server).await;
    let list = rpc(&server, "tools/list", Value::Null, None).await;
    let names = tool_names(&list);
    assert_tool_names(
        &names,
        &[
            "GatewayBatch",
            "GatewayExecute",
            "GatewayIndex",
            "GatewayIntent",
            "GetHelp",
            "ReadMcpResource",
            "complete_node",
        ],
    );

    let index = rpc(
        &server,
        "tools/call",
        json!({"name":"GatewayIndex","arguments":{}}),
        None,
    )
    .await;
    let index_text = result_text(&index);
    assert!(
        index_text.contains("\"gatewayStatus\": \"degraded\""),
        "{index_text}"
    );
    assert!(
        index_text.contains("started degraded or exposed no tools")
            || index_text.contains("gateway startup failed"),
        "{index_text}"
    );

    let intent = rpc(
        &server,
        "tools/call",
        json!({"name":"GatewayIntent","arguments":{"intent":"inspect gateway"}}),
        None,
    )
    .await;
    let intent_text = result_text(&intent);
    assert!(intent_text.contains("\"status\""), "{intent_text}");
    assert!(intent_text.contains("GatewayIndex"), "{intent_text}");

    let diagnostics = rpc(
        &server,
        "tools/call",
        json!({"name":"ReadMcpResource","arguments":{"uri":"gateway://diagnostics"}}),
        None,
    )
    .await;
    let diagnostics_text = result_text(&diagnostics);
    assert!(
        diagnostics_text.contains("broken-server") && diagnostics_text.contains("failed"),
        "{diagnostics_text}"
    );

    server.stop().await;
}

#[tokio::test]
#[serial]
async fn streamable_http_gateway_intent_enabled_lists_current_tools() {
    let mut config = gateway_config(vec![stdio_server(
        "small-server",
        vec![ok_tool("small_ping", "pong")],
    )]);
    config["agents"] = json!({
        "resolver": {"command": "/bin/echo", "env": {}, "mcpServers": {}}
    });
    config["settings"] = json!({
        "gateway": {"intentResolver": {"enabled": true, "agent": "resolver"}}
    });
    let server = TestServer::start(config).await;

    initialize(&server).await;
    let list = rpc(&server, "tools/list", Value::Null, None).await;
    let names = tool_names(&list);
    assert_tool_names(
        &names,
        &[
            "GatewayBatch",
            "GatewayExecute",
            "GatewayIndex",
            "GatewayIntent",
            "GetHelp",
            "ReadMcpResource",
            "complete_node",
        ],
    );

    server.stop().await;
}

fn assert_tool_names(actual: &[String], expected: &[&str]) {
    let mut actual = actual.to_vec();
    actual.sort();
    let mut expected: Vec<String> = expected.iter().map(|name| (*name).to_string()).collect();
    expected.sort();
    assert_eq!(actual, expected);
}

fn assert_tool_description(tools: &[Value], name: &str, expected: &str) {
    let description = tools
        .iter()
        .find(|tool| tool["name"] == name)
        .and_then(|tool| tool["description"].as_str())
        .expect("tool description");
    assert!(
        description.contains(expected),
        "{name} description did not contain {expected:?}: {description}"
    );
}
