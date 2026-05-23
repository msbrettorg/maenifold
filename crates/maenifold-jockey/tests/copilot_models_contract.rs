use std::{collections::HashMap, fs};

use jockey::copilot_models::{CopilotModelsCacheEntry, CopilotModelsEnvelope};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

struct MockResponse {
    status_line: String,
    headers: Vec<(String, String)>,
    body: String,
}

struct CapturedRequest {
    path: String,
    headers: HashMap<String, String>,
}

async fn serve_once(response: MockResponse) -> (String, tokio::task::JoinHandle<CapturedRequest>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock listener");
    let endpoint = format!("http://{}", listener.local_addr().expect("listener addr"));
    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept request");
        let mut buffer = vec![0_u8; 8192];
        let mut request = Vec::new();
        loop {
            let bytes = stream.read(&mut buffer).await.expect("read request");
            assert!(bytes > 0, "request closed before headers completed");
            request.extend_from_slice(&buffer[..bytes]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        let request_text = String::from_utf8(request).expect("request utf8");
        let mut lines = request_text.split("\r\n");
        let request_line = lines.next().expect("request line");
        let path = request_line
            .split_whitespace()
            .nth(1)
            .expect("request path")
            .to_string();
        let headers = lines
            .take_while(|line| !line.is_empty())
            .filter_map(|line| line.split_once(':'))
            .map(|(name, value)| (name.to_ascii_lowercase(), value.trim().to_string()))
            .collect::<HashMap<_, _>>();
        let mut response_text = format!(
            "HTTP/1.1 {}\r\nContent-Length: {}\r\nConnection: close\r\n",
            response.status_line,
            response.body.len()
        );
        for (name, value) in response.headers {
            response_text.push_str(&format!("{name}: {value}\r\n"));
        }
        response_text.push_str("\r\n");
        response_text.push_str(&response.body);
        stream
            .write_all(response_text.as_bytes())
            .await
            .expect("write response");
        CapturedRequest { path, headers }
    });
    (endpoint, handle)
}

fn fixture_body() -> String {
    fs::read_to_string("tests/fixtures/copilot-models.json").expect("read fixture")
}

mod client_contract {
    pub use jockey::copilot_models::{CopilotModel, CopilotModelsEnvelope};

    #[allow(dead_code)]
    mod inner {
        include!("../src/copilot_models/client.rs");

        async fn assert_status_maps(
            status_line: &'static str,
            expected: fn(&CopilotModelsClientError) -> bool,
        ) {
            let (endpoint, handle) = super::super::serve_once(super::super::MockResponse {
                status_line: status_line.to_string(),
                headers: vec![],
                body: "{}".to_string(),
            })
            .await;
            let client = CopilotModelsClient::new(build_http_client().expect("http"));
            let error = match client.fetch_models(&endpoint, "secret-token").await {
                Err(error) => error,
                Ok(_) => panic!("non-success status should fail"),
            };
            assert!(expected(&error), "unexpected error variant: {error:?}");
            let _ = handle.await.expect("server join");
        }

        fn assert_ttl_lock(
            endpoint: String,
            cached_at: chrono::DateTime<chrono::Utc>,
            models: Vec<CopilotModel>,
        ) -> super::super::CopilotModelsCacheEntry {
            super::super::CopilotModelsCacheEntry {
                endpoint,
                fingerprint: "fingerprint".to_string(),
                cached_at,
                expires_at: cached_at + chrono::Duration::hours(6),
                models,
            }
        }

        #[tokio::test]
        async fn fetch_models_sends_zed_parity_headers() {
            let (endpoint, handle) = super::super::serve_once(super::super::MockResponse {
                status_line: "200 OK".to_string(),
                headers: vec![],
                body: super::super::fixture_body(),
            })
            .await;
            let client = CopilotModelsClient::new(build_http_client().expect("http"));
            client
                .fetch_models(&endpoint, "secret-token")
                .await
                .expect("fetch models");
            let captured = handle.await.expect("server join");
            assert_eq!(captured.path, "/models");
            assert_eq!(
                captured.headers.get("authorization").map(String::as_str),
                Some("Bearer secret-token")
            );
            assert_eq!(
                captured.headers.get("editor-version").map(String::as_str),
                Some(concat!("maenifold/", env!("CARGO_PKG_VERSION")))
            );
            assert_eq!(
                captured
                    .headers
                    .get("x-github-api-version")
                    .map(String::as_str),
                Some("2025-04-01")
            );
        }

        #[tokio::test]
        async fn fetch_models_maps_401_to_unauthorized() {
            assert_status_maps("401 Unauthorized", |error| {
                matches!(error, CopilotModelsClientError::Unauthorized)
            })
            .await;
        }

        #[tokio::test]
        async fn fetch_models_maps_403_to_forbidden() {
            assert_status_maps("403 Forbidden", |error| {
                matches!(error, CopilotModelsClientError::Forbidden)
            })
            .await;
        }

        #[tokio::test]
        async fn fetch_models_maps_404_to_endpoint_not_found() {
            assert_status_maps("404 Not Found", |error| {
                matches!(error, CopilotModelsClientError::EndpointNotFound)
            })
            .await;
        }

        #[tokio::test]
        async fn fetch_models_maps_429_to_rate_limited() {
            assert_status_maps("429 Too Many Requests", |error| {
                matches!(error, CopilotModelsClientError::RateLimited)
            })
            .await;
        }

        #[tokio::test]
        async fn fetch_models_maps_500_to_server_error() {
            assert_status_maps("500 Internal Server Error", |error| {
                matches!(
                    error,
                    CopilotModelsClientError::ServerError { status }
                    if *status == reqwest::StatusCode::INTERNAL_SERVER_ERROR
                )
            })
            .await;
        }

        #[tokio::test]
        async fn fetch_models_maps_other_status_to_unexpected_status() {
            assert_status_maps("418 I'm a teapot", |error| {
                matches!(
                    error,
                    CopilotModelsClientError::UnexpectedStatus { status }
                    if *status == reqwest::StatusCode::IM_A_TEAPOT
                )
            })
            .await;
        }

        async fn assert_cache_expiry(cache_control: Option<&'static str>) {
            let mut headers = Vec::new();
            if let Some(value) = cache_control {
                headers.push(("Cache-Control".to_string(), value.to_string()));
            }
            let (endpoint, handle) = super::super::serve_once(super::super::MockResponse {
                status_line: "200 OK".to_string(),
                headers,
                body: super::super::fixture_body(),
            })
            .await;
            let client = CopilotModelsClient::new(build_http_client().expect("http"));
            let fetch = client
                .fetch_models(&endpoint, "secret-token")
                .await
                .expect("fetch models");
            let cached_at = chrono::Utc::now();
            let entry = assert_ttl_lock(endpoint.clone(), cached_at, fetch.models);
            assert_eq!(entry.expires_at, cached_at + chrono::Duration::hours(6));
            let _ = handle.await.expect("server join");
        }

        #[tokio::test]
        async fn cache_expiry_stays_at_six_hours_when_server_returns_short_max_age() {
            assert_cache_expiry(Some("private, max-age=300")).await;
        }

        #[tokio::test]
        async fn cache_expiry_stays_at_six_hours_when_cache_control_missing() {
            assert_cache_expiry(None).await;
        }
    }
}

#[test]
fn fixture_still_parses_into_public_envelope_type() {
    let envelope: CopilotModelsEnvelope =
        serde_json::from_str(&fixture_body()).expect("parse fixture envelope");
    assert!(!envelope.data.is_empty());
}
