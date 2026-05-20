use futures_util::stream::BoxStream;
use reqwest::header::{HeaderName, HeaderValue};
use rmcp::transport::{
    auth::{AuthClient, AuthError, ClientCredentialsConfig},
    streamable_http_client::{
        StreamableHttpClient, StreamableHttpError, StreamableHttpPostResponse,
    },
};
use sse_stream::{Error as SseError, Sse};
use std::{collections::HashMap, sync::Arc};

#[derive(Clone)]
pub(super) enum GatewayHttpClient {
    Plain(reqwest::Client),
    OAuth(OAuthHttpClient),
}

#[derive(Clone)]
pub(super) struct OAuthHttpClient {
    pub(super) inner: AuthClient<reqwest::Client>,
    pub(super) client_credentials: ClientCredentialsConfig,
}

impl OAuthHttpClient {
    pub(super) async fn current_token(
        &self,
    ) -> std::result::Result<String, StreamableHttpError<reqwest::Error>> {
        let manager = self.inner.auth_manager.lock().await;
        match manager.get_access_token().await {
            Ok(token) => Ok(token),
            Err(AuthError::AuthorizationRequired | AuthError::TokenRefreshFailed(_)) => {
                manager
                    .exchange_client_credentials(&self.client_credentials)
                    .await?;
                manager
                    .get_access_token()
                    .await
                    .map_err(StreamableHttpError::Auth)
            }
            Err(error) => Err(StreamableHttpError::Auth(error)),
        }
    }

    async fn refresh_token(
        &self,
    ) -> std::result::Result<String, StreamableHttpError<reqwest::Error>> {
        let manager = self.inner.auth_manager.lock().await;
        manager
            .exchange_client_credentials(&self.client_credentials)
            .await?;
        manager
            .get_access_token()
            .await
            .map_err(StreamableHttpError::Auth)
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
            Self::OAuth(client) => {
                let token = client.current_token().await?;
                let result = client
                    .inner
                    .http_client
                    .post_message(
                        uri.clone(),
                        message.clone(),
                        session_id.clone(),
                        Some(token),
                        custom_headers.clone(),
                    )
                    .await;
                if should_reauthenticate(&result) {
                    let token = client.refresh_token().await?;
                    return client
                        .inner
                        .http_client
                        .post_message(uri, message, session_id, Some(token), custom_headers)
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
            Self::OAuth(client) => {
                let token = client.current_token().await?;
                let result = client
                    .inner
                    .http_client
                    .delete_session(
                        uri.clone(),
                        session_id.clone(),
                        Some(token),
                        custom_headers.clone(),
                    )
                    .await;
                if should_reauthenticate(&result) {
                    let token = client.refresh_token().await?;
                    return client
                        .inner
                        .http_client
                        .delete_session(uri, session_id, Some(token), custom_headers)
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
            Self::OAuth(client) => {
                let token = client.current_token().await?;
                let result = client
                    .inner
                    .http_client
                    .get_stream(
                        uri.clone(),
                        session_id.clone(),
                        last_event_id.clone(),
                        Some(token),
                        custom_headers.clone(),
                    )
                    .await;
                if should_reauthenticate(&result) {
                    let token = client.refresh_token().await?;
                    return client
                        .inner
                        .http_client
                        .get_stream(uri, session_id, last_event_id, Some(token), custom_headers)
                        .await;
                }
                result
            }
        }
    }
}

fn should_reauthenticate<T>(
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
