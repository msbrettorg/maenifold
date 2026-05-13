use crate::mcp::MaenifoldClient;
use rmcp::{
    ServerHandler, ServiceExt,
    model::{
        ClientResult, CreateElicitationRequest, CreateElicitationRequestParams, ElicitationAction,
        ElicitationSchema, PrimitiveSchema, ServerCapabilities, ServerInfo, ServerRequest,
        StringSchema,
    },
};

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

    let mut client = MaenifoldClient::new().serve(client_transport).await?;
    let info = info_rx.await?;
    stop_tx
        .send(())
        .map_err(|_| anyhow::anyhow!("failed to stop test server"))?;
    server_task.await??;
    client.close().await?;

    assert!(info.capabilities.elicitation.is_some());
    assert!(
        info.capabilities
            .tasks
            .as_ref()
            .is_some_and(|tasks| tasks.supports_elicitation_create())
    );
    Ok(())
}

#[tokio::test]
async fn elicitation_create_returns_cancel() -> anyhow::Result<()> {
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

    let mut client = MaenifoldClient::new().serve(client_transport).await?;
    let result = server_task.await??;
    client.close().await?;

    assert_eq!(result.action, ElicitationAction::Cancel);
    assert!(result.content.is_none());
    Ok(())
}
