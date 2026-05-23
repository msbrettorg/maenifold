mod common;

use common::gateway_http::{
    TestServer, gateway_config, initialize, result_text, rpc, stdio_server, tool_names,
};
use maenifold_core::Config;
use serde_json::{Value, json};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn gateway_index_includes_gateway_servers_added_after_startup() {
    let server = TestServer::start(gateway_config(vec![stdio_server(
        "alpha",
        vec![json!({"name": "alpha_ping", "description": "alpha ping", "text": "alpha"})],
    )]))
    .await;

    initialize(&server).await;

    let initial = gateway_index(&server).await;
    assert!(initial.contains("\"name\": \"alpha\""));
    assert!(!initial.contains("\"name\": \"beta\""));

    let updated = gateway_config(vec![
        stdio_server(
            "alpha",
            vec![json!({"name": "alpha_ping", "description": "alpha ping", "text": "alpha"})],
        ),
        stdio_server(
            "beta",
            vec![json!({"name": "beta_ping", "description": "beta ping", "text": "beta"})],
        ),
    ]);
    std::fs::write(
        Config::current().maenifold_root.join("maenifold.json"),
        serde_json::to_vec_pretty(&updated).expect("updated config should serialize"),
    )
    .expect("updated config should write");

    let rediscovered = gateway_index(&server).await;
    assert!(rediscovered.contains("\"name\": \"beta\""));

    let names = tool_names(&rpc(&server, "tools/list", Value::Null, None).await);
    assert_eq!(
        names,
        vec![
            "GatewayBatch",
            "GatewayExecute",
            "GatewayIndex",
            "GetHelp",
            "ReadMcpResource",
            "complete_node",
        ]
    );

    server.stop().await;
}

async fn gateway_index(server: &TestServer) -> String {
    let response = rpc(
        server,
        "tools/call",
        json!({"name":"GatewayIndex","arguments":{}}),
        None,
    )
    .await;
    result_text(&response).to_string()
}
