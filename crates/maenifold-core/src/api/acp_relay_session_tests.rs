use super::*;

#[test]
fn rejects_session_new_and_load_client_mcp_server_injection() {
    for (method, request_id) in [("session/new", json!(7)), ("session/load", json!("load"))] {
        let error = reject_client_mcp_servers(
            &json!({
                "sessionId": "session-1",
                "mcpServers": [{
                    "type": "stdio",
                    "name": "evil",
                    "command": "malicious"
                }]
            }),
            request_id.clone(),
            method,
        )
        .expect_err("client mcpServers must be rejected");
        assert_eq!(error["id"], request_id);
        assert_eq!(error["error"]["code"], json!(-32602));
        assert_eq!(
            error["error"]["message"],
            format!("{method} does not accept client-provided mcpServers")
        );
    }
}

fn relay_for(agent_name: &str) -> RelayConfig {
    RelayConfig {
        agent_name: agent_name.to_string(),
        adapter: jockey::acp::StdioAdapterSpec {
            runtime_key: agent_name.to_string(),
            binary: agent_name.to_string(),
            args: Vec::new(),
            env: Vec::new(),
        },
        default_cwd: ".".to_string(),
        configured_mcp_servers: Vec::new(),
    }
}

#[test]
fn initialize_handshake_for_claude_reports_image_not_audio() {
    let relay = relay_for("claude");
    let payload = initialize_response(&relay, caps_for_agent(&relay.agent_name, &relay.adapter));
    let caps = &payload["agentCapabilities"]["promptCapabilities"];
    assert_eq!(caps["image"], json!(true));
    assert_eq!(caps["audio"], json!(false));
    assert_eq!(caps["embeddedContext"], json!(true));
}

#[test]
fn initialize_handshake_for_ollama_reports_no_image_or_audio() {
    let relay = relay_for("ollama-llama3");
    let payload = initialize_response(&relay, caps_for_agent(&relay.agent_name, &relay.adapter));
    let caps = &payload["agentCapabilities"]["promptCapabilities"];
    assert_eq!(caps["image"], json!(false));
    assert_eq!(caps["audio"], json!(false));
    assert_eq!(caps["embeddedContext"], json!(true));
}
