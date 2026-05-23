mod common;

use common::gateway_http::{TestServer, gateway_config, initialize, ok_tool, rpc, stdio_server, tool_names};
use serde_json::{Value, json};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn wire_surface_exposes_only_canonical_seven_tools() {
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
    let mut actual = tool_names(&list);
    actual.sort();

    let expected = canonical_tool_names();
    assert_eq!(actual, expected);

    for forbidden in [
        "DiscoverMcpPrompts",
        "DiscoverMcpTools",
        "GetMcpPrompt",
    ] {
        assert!(
            !actual.iter().any(|name| name == forbidden),
            "forbidden tool {forbidden} was still exposed: {actual:?}"
        );
    }

    server.stop().await;
}

fn canonical_tool_names() -> Vec<String> {
    [
        "GatewayBatch",
        "GatewayExecute",
        "GatewayIndex",
        "GatewayIntent",
        "GetHelp",
        "ReadMcpResource",
        "complete_node",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}
