use futures_util::stream::BoxStream;
use reqwest::header::{HeaderName, HeaderValue};
use rmcp::transport::streamable_http_client::{
    StreamableHttpClient, StreamableHttpError, StreamableHttpPostResponse,
};
use sse_stream::{Error as SseError, Sse};
use std::borrow::Cow;
use std::{collections::HashMap, sync::Arc};

use super::http_oauth::AzureDefaultCredential;

#[derive(Clone)]
pub(super) enum GatewayHttpClient {
    Plain(reqwest::Client),
    Azure(AzureHttpClient),
}

#[derive(Clone)]
pub(super) struct AzureHttpClient {
    http_client: reqwest::Client,
    credential: AzureDefaultCredential,
    scope: Arc<str>,
}

impl AzureHttpClient {
    pub(super) fn new(
        http_client: reqwest::Client,
        credential: AzureDefaultCredential,
        scope: String,
    ) -> Self {
        Self {
            http_client,
            credential,
            scope: Arc::<str>::from(scope),
        }
    }

    async fn current_token(
        &self,
    ) -> std::result::Result<String, StreamableHttpError<reqwest::Error>> {
        self.credential
            .get_token(self.scope.as_ref())
            .await
            .map_err(|error| {
                StreamableHttpError::UnexpectedServerResponse(Cow::Owned(error.to_string()))
            })
    }
}

impl StreamableHttpClient for GatewayHttpClient {
    type Error = reqwest::Error;

    async fn post_message(
        &self,
        uri: Arc<str>,
        message: rmcp::model::ClientJsonRpcMessage,
        session_id: Option<Arc<str>>,
        auth_token: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> std::result::Result<StreamableHttpPostResponse, StreamableHttpError<Self::Error>> {
        match self {
            Self::Plain(client) => {
                client
                    .post_message(uri, message, session_id, auth_token, custom_headers)
                    .await
            }
            Self::Azure(client) => {
                let token = client.current_token().await?;
                let result = client
                    .http_client
                    .post_message(
                        uri.clone(),
                        message.clone(),
                        session_id.clone(),
                        Some(token),
                        custom_headers.clone(),
                    )
                    .await;
                if should_retry_with_fresh_token(&result) {
                    return client
                        .http_client
                        .post_message(
                            uri,
                            message,
                            session_id,
                            Some(client.current_token().await?),
                            custom_headers,
                        )
                        .await;
                }
                result
            }
        }
    }

    async fn delete_session(
        &self,
        uri: Arc<str>,
        session_id: Arc<str>,
        auth_token: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> std::result::Result<(), StreamableHttpError<Self::Error>> {
        match self {
            Self::Plain(client) => {
                client
                    .delete_session(uri, session_id, auth_token, custom_headers)
                    .await
            }
            Self::Azure(client) => {
                let token = client.current_token().await?;
                let result = client
                    .http_client
                    .delete_session(
                        uri.clone(),
                        session_id.clone(),
                        Some(token),
                        custom_headers.clone(),
                    )
                    .await;
                if should_retry_with_fresh_token(&result) {
                    return client
                        .http_client
                        .delete_session(
                            uri,
                            session_id,
                            Some(client.current_token().await?),
                            custom_headers,
                        )
                        .await;
                }
                result
            }
        }
    }

    async fn get_stream(
        &self,
        uri: Arc<str>,
        session_id: Arc<str>,
        last_event_id: Option<String>,
        auth_token: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> std::result::Result<
        BoxStream<'static, std::result::Result<Sse, SseError>>,
        StreamableHttpError<Self::Error>,
    > {
        match self {
            Self::Plain(client) => {
                client
                    .get_stream(uri, session_id, last_event_id, auth_token, custom_headers)
                    .await
            }
            Self::Azure(client) => {
                let token = client.current_token().await?;
                let result = client
                    .http_client
                    .get_stream(
                        uri.clone(),
                        session_id.clone(),
                        last_event_id.clone(),
                        Some(token),
                        custom_headers.clone(),
                    )
                    .await;
                if should_retry_with_fresh_token(&result) {
                    return client
                        .http_client
                        .get_stream(
                            uri,
                            session_id,
                            last_event_id,
                            Some(client.current_token().await?),
                            custom_headers,
                        )
                        .await;
                }
                result
            }
        }
    }
}

fn should_retry_with_fresh_token<T>(
    result: &std::result::Result<T, StreamableHttpError<reqwest::Error>>,
) -> bool {
    match result {
        Err(StreamableHttpError::AuthRequired(_)) => true,
        Err(StreamableHttpError::Client(error)) => {
            error.status() == Some(reqwest::StatusCode::UNAUTHORIZED)
        }
        _ => false,
    }
}
