use super::resources_graph_corpus_primer_test_support::*;
use super::resources_tests_common::*;
use crate::mcp::MaenifoldMcpServer;
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

const PROTOCOL_VERSION: &str = "2025-03-26";

struct TestHttpServer {
    endpoint: String,
    shutdown: CancellationToken,
    task: JoinHandle<crate::Result<()>>,
}

impl TestHttpServer {
    async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral MCP listener");
        let endpoint = format!(
            "http://{}/mcp",
            listener.local_addr().expect("listener addr")
        );
        let shutdown = CancellationToken::new();
        let task_shutdown = shutdown.clone();
        let task = tokio::spawn(async move {
            MaenifoldMcpServer::serve_http_listener_with_shutdown(listener, task_shutdown).await
        });

        let server = Self {
            endpoint,
            shutdown,
            task,
        };
        server.wait_until_ready().await;
        server
    }

    async fn wait_until_ready(&self) {
        let client = reqwest::Client::new();
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            if tokio::time::Instant::now() > deadline {
                panic!("MCP HTTP listener did not become ready");
            }
            if client.get(&self.endpoint).send().await.is_ok() {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    }

    async fn stop(self) {
        self.shutdown.cancel();
        let result = tokio::time::timeout(std::time::Duration::from_secs(8), self.task)
            .await
            .expect("server shutdown should complete")
            .expect("server task should not panic");
        result.expect("server should stop cleanly");
    }
}

fn headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/json, text/event-stream"),
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        "mcp-protocol-version",
        HeaderValue::from_static(PROTOCOL_VERSION),
    );
    headers
}

async fn rpc(server: &TestHttpServer, method: &str, params: Value) -> Value {
    let body = json!({"jsonrpc":"2.0","id":1,"method":method,"params":params});
    let response = reqwest::Client::new()
        .post(&server.endpoint)
        .headers(headers())
        .json(&body)
        .send()
        .await
        .expect("send MCP request");
    response.json().await.expect("decode JSON-RPC response")
}

#[tokio::test]
#[serial]
async fn graph_corpus_primer_http_read_returns_application_json() {
    let (_tmp, _guard) = setup_primer_fixture();
    let server = TestHttpServer::start().await;

    let _ = rpc(
        &server,
        "initialize",
        json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": {"name": "graph-corpus-primer-test", "version": "0.0.1"}
        }),
    )
    .await;

    let response = rpc(&server, "resources/read", json!({ "uri": PRIMER_URI })).await;

    assert!(
        response.get("error").is_none(),
        "primer resources/read should succeed: {response}"
    );
    assert_eq!(
        response["result"]["contents"][0]["mimeType"],
        json!("application/json")
    );
    let text = response["result"]["contents"][0]["text"]
        .as_str()
        .expect("primer text payload");
    assert_payload_lines(text, &expected_lines());

    server.stop().await;
}
