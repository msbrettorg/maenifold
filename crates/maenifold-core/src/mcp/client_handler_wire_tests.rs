use crate::mcp::{MaenifoldClient, PendingRequestRegistry};
use rmcp::ServiceExt;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Asserts that the MCP initialize request sent by MaenifoldClient on the wire
/// includes `capabilities.sampling` and `capabilities.sampling.tools` as objects.
/// rmcp uses newline-delimited JSON framing (one JSON object per line).
#[tokio::test]
async fn initialize_request_wire_format_includes_sampling_capability() -> anyhow::Result<()> {
    let (server_transport, client_transport) = tokio::io::duplex(8192);
    let client_task = tokio::spawn(async move {
        MaenifoldClient::new(Arc::new(PendingRequestRegistry::new()))
            .serve(client_transport)
            .await
    });
    let mut reader = BufReader::new(server_transport);

    // Read one newline-delimited JSON frame (the initialize request)
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    let initialize: Value = serde_json::from_str(line.trim())?;

    assert_eq!(initialize["jsonrpc"], "2.0");
    assert_eq!(initialize["method"], "initialize");
    assert!(
        initialize
            .pointer("/params/capabilities/sampling")
            .and_then(Value::as_object)
            .is_some(),
        "wire initialize request must include params.capabilities.sampling object: {initialize}"
    );
    assert!(
        initialize
            .pointer("/params/capabilities/sampling/tools")
            .and_then(Value::as_object)
            .is_some(),
        "wire initialize request must include params.capabilities.sampling.tools object: {initialize}"
    );
    assert!(
        initialize
            .pointer("/params/capabilities/elicitation")
            .and_then(Value::as_object)
            .is_some(),
        "wire initialize request must include params.capabilities.elicitation object: {initialize}"
    );

    // Send back a valid initialize response so the client doesn't hang
    let response = json!({
        "jsonrpc": "2.0",
        "id": initialize["id"].clone(),
        "result": {
            "protocolVersion": "2025-03-26",
            "capabilities": {},
            "serverInfo": {
                "name": "wire-test-server",
                "version": "0.0.1"
            }
        }
    });
    let inner = reader.into_inner();
    let response_bytes = serde_json::to_vec(&response)?;
    let (_read_half, mut write_half) = tokio::io::split(inner);
    write_half.write_all(&response_bytes).await?;
    write_half.write_all(b"\n").await?;
    write_half.flush().await?;

    let mut client = client_task.await??;
    client.close().await?;
    Ok(())
}
