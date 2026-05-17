// TDD ANCHOR: these tests assert the cross-cutting sampling/createMessage
// dispatch contract spec'd in T-303.4.7. They are designed to fail
// compilation today; the SWE will introduce:
//   - MaenifoldClient::new_with_sampling (or builder method)
//   - error code path -32603 when samplingAgent unconfigured
//   - error code path -32602 when sampling/createMessage carries tools
//   - error code path -1 when user denies via UserPromptResponse::Sampling
//
// Happy-path approve test belongs to the SWE delta — they need to wire
// `execute_runtime_sampling` first.

use crate::mcp::MaenifoldClient;
use crate::mcp::pending_requests::{
    PendingRequestRegistry, SamplingAction, SamplingDecision, UserPromptResponse,
};
use rmcp::{
    ServerHandler, ServiceExt,
    model::{
        ClientResult, CreateMessageRequest, CreateMessageRequestParams, ErrorData, SamplingMessage,
        ServerCapabilities, ServerInfo, ServerRequest,
    },
    service::ServiceError,
};
use std::sync::Arc;
use std::time::Duration;

struct TestServer;

impl ServerHandler for TestServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::default())
    }
}

#[tokio::test]
async fn sampling_with_unconfigured_sampling_agent_rejects_negative_32603() -> anyhow::Result<()> {
    let error = send_sampling_request(valid_sampling_params(), None, None).await?;

    assert_eq!(error.code.0, -32603);
    Ok(())
}

#[test]
fn sampling_with_tools_field_uses_tool_enabled_runtime_flow_source_contract() {
    let handler_source = include_str!("client_handler.rs");
    let flow_source = include_str!("sampling_client_flow.rs");

    assert!(
        handler_source.contains("enable_sampling_tools"),
        "T-303.4.5: MCP client capabilities must advertise sampling tool support"
    );
    assert!(
        flow_source.contains("request.tools.clone()"),
        "T-303.4.5: sampling/createMessage tools must be passed into runtime sampling"
    );
}

#[tokio::test]
async fn sampling_deny_returns_negative_1() -> anyhow::Result<()> {
    let registry = Arc::new(PendingRequestRegistry::new());
    let registry_for_response = Arc::clone(&registry);
    let (server_transport, client_transport) = tokio::io::duplex(4096);

    let server_task = tokio::spawn(async move {
        let mut server = TestServer.serve(server_transport).await?;
        let result = server
            .peer()
            .send_request(ServerRequest::CreateMessageRequest(
                CreateMessageRequest::new(valid_sampling_params()),
            ))
            .await;
        let error = expect_mcp_error(result)?;
        server.close().await?;
        anyhow::Ok(error)
    });

    let mut client = client_with_sampling_agent(registry, Some("copilot".to_string()))
        .serve(client_transport)
        .await?;
    let id = wait_for_sampling_pending_id(&registry_for_response).await?;
    registry_for_response
        .respond_typed(UserPromptResponse::Sampling {
            id,
            response: SamplingDecision {
                action: SamplingAction::Deny,
                selected_model: None,
                selected_agent: None,
            },
        })
        .await
        .map_err(|error| anyhow::anyhow!("respond failed: {error:?}"))?;

    let error = server_task.await??;
    client.close().await?;

    assert_eq!(error.code.0, -1);
    Ok(())
}

#[test]
fn sampling_allow_dispatch_passes_selected_model_to_runtime_sampling() {
    let source = include_str!("sampling_client_flow.rs");
    let dispatch_start = source
        .find("jockey::acp::execute_runtime_sampling")
        .expect("client_handler must dispatch approved sampling through execute_runtime_sampling");
    let dispatch = &source[dispatch_start..];
    let dispatch_end = dispatch
        .find(".await")
        .expect("execute_runtime_sampling call must be awaited");
    let call = &dispatch[..dispatch_end];

    // Format-tolerant: strip whitespace and check the model expression is passed through.
    let call_no_ws: String = call.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        call_no_ws.contains("decision.selected_model.clone()"),
        "T-303.4.19: approved sampling must pass decision.selected_model.clone() to execute_runtime_sampling"
    );
}

async fn send_sampling_request(
    params: CreateMessageRequestParams,
    sampling_agent: Option<String>,
    registry: Option<Arc<PendingRequestRegistry>>,
) -> anyhow::Result<ErrorData> {
    let (server_transport, client_transport) = tokio::io::duplex(4096);
    let server_task = tokio::spawn(async move {
        let mut server = TestServer.serve(server_transport).await?;
        let result = server
            .peer()
            .send_request(ServerRequest::CreateMessageRequest(
                CreateMessageRequest::new(params),
            ))
            .await;
        let error = expect_mcp_error(result)?;
        server.close().await?;
        anyhow::Ok(error)
    });

    let registry = registry.unwrap_or_else(|| Arc::new(PendingRequestRegistry::new()));
    let mut client = client_with_sampling_agent(registry, sampling_agent)
        .serve(client_transport)
        .await?;
    let error = server_task.await??;
    client.close().await?;
    Ok(error)
}

fn client_with_sampling_agent(
    registry: Arc<PendingRequestRegistry>,
    sampling_agent: Option<String>,
) -> MaenifoldClient {
    MaenifoldClient::new_with_sampling(registry, sampling_agent)
}

fn valid_sampling_params() -> CreateMessageRequestParams {
    CreateMessageRequestParams::new(
        vec![SamplingMessage::user_text("Summarize this context")],
        64,
    )
}

fn expect_mcp_error(result: Result<ClientResult, ServiceError>) -> anyhow::Result<ErrorData> {
    match result {
        Err(ServiceError::McpError(error)) => Ok(error),
        Err(other) => anyhow::bail!("unexpected service error: {other:?}"),
        Ok(other) => anyhow::bail!("expected JSON-RPC error, got response: {other:?}"),
    }
}

async fn wait_for_sampling_pending_id(
    registry: &PendingRequestRegistry,
) -> anyhow::Result<uuid::Uuid> {
    for _ in 0..50 {
        let events = registry.pending_events().await;
        if let Some(event) = events.into_iter().find(|event| event.kind == "sampling") {
            return Ok(event.id);
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    anyhow::bail!("timed out waiting for pending sampling request");
}
