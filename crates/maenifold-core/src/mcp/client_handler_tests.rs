use crate::mcp::MaenifoldClient;
use crate::mcp::PendingRequestRegistry;
use crate::mcp::pending_requests::{ElicitationResponse, UserPromptAction, UserPromptResponse};
use rmcp::{
    ServerHandler, ServiceExt,
    model::{
        ClientResult, CreateElicitationRequest, CreateElicitationRequestParams,
        CreateElicitationResult,
        CreateMessageRequest, CreateMessageRequestParams, ElicitationAction, ElicitationSchema,
        PrimitiveSchema, SamplingMessage, ServerCapabilities, ServerInfo, ServerRequest,
        StringSchema,
    },
    service::ServiceError,
};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

struct TestServer;

impl ServerHandler for TestServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::default())
    }
}

#[tokio::test]
async fn advertises_elicitation_capability() -> anyhow::Result<()> {
    let (server_transport, client_transport) = tokio::io::duplex(4096);
    let (info_tx, info_rx) = tokio::sync::oneshot::channel();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();

    let server_task = tokio::spawn(async move {
        let mut server = TestServer.serve(server_transport).await?;
        let info = server
            .peer()
            .peer_info()
            .cloned()
            .expect("client info should be recorded after initialize");
        info_tx
            .send(info)
            .map_err(|_| anyhow::anyhow!("failed to send client info"))?;
        stop_rx.await?;
        server.close().await?;
        anyhow::Ok(())
    });

    let registry = Arc::new(PendingRequestRegistry::new());
    let mut client = MaenifoldClient::new(registry).serve(client_transport).await?;
    let info = info_rx.await?;
    stop_tx
        .send(())
        .map_err(|_| anyhow::anyhow!("failed to stop test server"))?;
    server_task.await??;
    client.close().await?;

    assert!(info.capabilities.elicitation.is_some());
    assert!(info.capabilities.sampling.is_some());
    assert!(
        info.capabilities
            .tasks
            .as_ref()
            .is_some_and(|tasks| tasks.supports_elicitation_create())
    );
    Ok(())
}

#[tokio::test]
async fn elicitation_create_returns_cancel_without_renderer() -> anyhow::Result<()> {
    let (server_transport, client_transport) = tokio::io::duplex(4096);

    let server_task = tokio::spawn(async move {
        let mut server = TestServer.serve(server_transport).await?;
        let response = server
            .peer()
            .send_request(ServerRequest::CreateElicitationRequest(
                CreateElicitationRequest::new(
                    CreateElicitationRequestParams::FormElicitationParams {
                        meta: None,
                        message: "Need approval".to_string(),
                        requested_schema: ElicitationSchema::builder()
                            .property("reason", PrimitiveSchema::String(StringSchema::new()))
                            .build()
                            .expect("schema should be valid"),
                    },
                ),
            ))
            .await?;
        server.close().await?;
        match response {
            ClientResult::CreateElicitationResult(result) => anyhow::Ok(result),
            other => anyhow::bail!("unexpected response: {other:?}"),
        }
    });

    let registry = Arc::new(PendingRequestRegistry::new());
    let mut client = MaenifoldClient::new(registry).serve(client_transport).await?;
    let result = server_task.await??;
    client.close().await?;

    assert_eq!(result.action, ElicitationAction::Cancel);
    assert!(result.content.is_none());
    Ok(())
}

#[tokio::test]
async fn elicitation_create_round_trips_registry_response() -> anyhow::Result<()> {
    let (server_transport, client_transport) = tokio::io::duplex(4096);
    let registry = Arc::new(PendingRequestRegistry::new());
    let registry_for_response = Arc::clone(&registry);

    let server_task = tokio::spawn(async move {
        let mut server = TestServer.serve(server_transport).await?;
        let response = server
            .peer()
            .send_request(ServerRequest::CreateElicitationRequest(
                CreateElicitationRequest::new(form_params()),
            ))
            .await?;
        server.close().await?;
        match response {
            ClientResult::CreateElicitationResult(result) => anyhow::Ok(result),
            other => anyhow::bail!("unexpected response: {other:?}"),
        }
    });

    let mut client = MaenifoldClient::new(registry).serve(client_transport).await?;
    let id = wait_for_pending_id(&registry_for_response).await?;
    registry_for_response
        .respond_typed(UserPromptResponse::Elicitation {
            id,
            response: ElicitationResponse {
                action: UserPromptAction::Accept,
                content: Some(json!({ "reason": "approved" })),
            },
        })
        .await
        .map_err(|error| anyhow::anyhow!("respond failed: {error:?}"))?;
    let result = server_task.await??;
    client.close().await?;

    assert_eq!(result.action, ElicitationAction::Accept);
    assert_eq!(result.content, Some(json!({ "reason": "approved" })));
    Ok(())
}

#[tokio::test]
async fn concurrent_elicitations_each_receive_matching_response() -> anyhow::Result<()> {
    let (server_transport, client_transport) = tokio::io::duplex(8192);
    let registry = Arc::new(PendingRequestRegistry::new());
    let registry_for_response = Arc::clone(&registry);

    let server_task = tokio::spawn(async move {
        let mut server = TestServer.serve(server_transport).await?;
        let first = server
            .peer()
            .send_request(ServerRequest::CreateElicitationRequest(
                CreateElicitationRequest::new(form_params()),
            ));
        let second = server
            .peer()
            .send_request(ServerRequest::CreateElicitationRequest(
                CreateElicitationRequest::new(form_params()),
            ));
        let (first, second) = tokio::try_join!(first, second)?;
        server.close().await?;
        Ok::<_, anyhow::Error>((extract_elicitation(first)?, extract_elicitation(second)?))
    });

    let mut client = MaenifoldClient::new(registry).serve(client_transport).await?;
    let ids = wait_for_pending_count(&registry_for_response, 2).await?;
    for (index, id) in ids.into_iter().enumerate() {
        registry_for_response
            .respond_typed(UserPromptResponse::Elicitation {
                id,
                response: ElicitationResponse {
                    action: UserPromptAction::Accept,
                    content: Some(json!({ "reason": format!("approved-{index}") })),
                },
            })
            .await
            .map_err(|error| anyhow::anyhow!("respond failed: {error:?}"))?;
    }
    let (first, second) = server_task.await??;
    client.close().await?;

    assert_eq!(first.action, ElicitationAction::Accept);
    assert_eq!(second.action, ElicitationAction::Accept);
    assert!(first.content.is_some());
    assert!(second.content.is_some());
    Ok(())
}

#[tokio::test]
async fn create_message_without_sampling_agent_returns_negative_32603() -> anyhow::Result<()> {
    let (server_transport, client_transport) = tokio::io::duplex(4096);

    let server_task = tokio::spawn(async move {
        let mut server = TestServer.serve(server_transport).await?;
        let result = server
            .peer()
            .send_request(ServerRequest::CreateMessageRequest(
                CreateMessageRequest::new(CreateMessageRequestParams::new(
                    vec![SamplingMessage::user_text("Summarize this context")],
                    64,
                )),
            ))
            .await;
        server.close().await?;
        match result {
            Err(ServiceError::McpError(error)) => anyhow::Ok(error),
            Err(other) => anyhow::bail!("unexpected service error: {other:?}"),
            Ok(other) => anyhow::bail!("expected JSON-RPC error, got response: {other:?}"),
        }
    });

    let registry = Arc::new(PendingRequestRegistry::new());
    let mut client = MaenifoldClient::new(registry).serve(client_transport).await?;
    let error = server_task.await??;
    client.close().await?;

    assert_eq!(error.code.0, -32603);
    Ok(())
}

fn form_params() -> CreateElicitationRequestParams {
    CreateElicitationRequestParams::FormElicitationParams {
        meta: None,
        message: "Need approval".to_string(),
        requested_schema: ElicitationSchema::builder()
            .mark_required("reason")
            .property("reason", PrimitiveSchema::String(StringSchema::new()))
            .build()
            .expect("schema should be valid"),
    }
}

fn extract_elicitation(response: ClientResult) -> anyhow::Result<CreateElicitationResult> {
    match response {
        ClientResult::CreateElicitationResult(result) => Ok(result),
        other => anyhow::bail!("unexpected response: {other:?}"),
    }
}

async fn wait_for_pending_id(registry: &PendingRequestRegistry) -> anyhow::Result<uuid::Uuid> {
    Ok(wait_for_pending_count(registry, 1).await?[0])
}

async fn wait_for_pending_count(
    registry: &PendingRequestRegistry,
    expected: usize,
) -> anyhow::Result<Vec<uuid::Uuid>> {
    for _ in 0..50 {
        let events = registry.pending_events().await;
        if events.len() >= expected {
            return Ok(events.into_iter().map(|event| event.id).collect());
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    anyhow::bail!("timed out waiting for pending elicitation");
}
