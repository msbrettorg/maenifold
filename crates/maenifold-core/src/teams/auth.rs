use std::collections::HashMap;
use std::time::Duration;

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use parking_lot::Mutex;
use serde::Deserialize;
use crate::error::{MaenifoldError, Result};
pub struct TeamsAuth {
    client: reqwest::Client,
    client_id: String,
    tenant_id: String,
    pending_flows: Mutex<HashMap<String, PendingDeviceFlow>>,
}
pub struct DeviceCodeInfo {
    pub user_code: String,
    pub verification_uri: String,
    pub message: String,
    pub expires_in: u64,
}
pub struct TokenInfo {
    pub access_token: String,
    pub expires_at: DateTime<Utc>,
    pub refresh_token: Option<String>,
}
#[derive(Clone)]
struct PendingDeviceFlow {
    device_code: String,
    user_code: String,
    interval: Duration,
    expires_at: DateTime<Utc>,
}
#[derive(Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    message: String,
    expires_in: u64,
    #[serde(default = "default_poll_interval")]
    interval: u64,
}
#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
    #[serde(default)]
    refresh_token: Option<String>,
}
#[derive(Deserialize)]
struct OAuthErrorResponse {
    #[serde(default)]
    error: String,
    #[serde(default)]
    error_description: String,
}
enum PollState {
    Pending,
    SlowDown,
    Failed(String),
}

impl TeamsAuth {
    pub fn new(client_id: String, tenant_id: String) -> Self {
        Self {
            client: reqwest::Client::builder().timeout(Duration::from_secs(30)).connect_timeout(Duration::from_secs(10)).build().expect("failed to build HTTP client"),
            client_id,
            tenant_id,
            pending_flows: Mutex::new(HashMap::new()),
        }
    }

    pub async fn initiate_device_code(&self, scopes: &[&str]) -> Result<DeviceCodeInfo> {
        let scope = join_scopes(scopes)?;
        let url = self.device_code_endpoint();
        let (status, body) = self
            .post_form(
                &url,
                &[("client_id", self.client_id.as_str()), ("scope", scope.as_str())],
                "device code request",
            )
            .await?;
        let response: DeviceCodeResponse = parse_json(&body, "device code response", status)?;
        let pending = PendingDeviceFlow {
            device_code: response.device_code.clone(),
            user_code: response.user_code.clone(),
            interval: Duration::from_secs(response.interval.max(1)),
            expires_at: Utc::now() + seconds_to_duration(response.expires_in),
        };
        self.store_pending_flow(pending);
        Ok(DeviceCodeInfo {
            user_code: response.user_code,
            verification_uri: response.verification_uri,
            message: response.message,
            expires_in: response.expires_in,
        })
    }

    pub async fn acquire_token(&self, device_code: &str) -> Result<TokenInfo> {
        let flow = self.pending_flow(device_code)?;
        let mut poll_interval = flow.interval;
        loop {
            if Utc::now() >= flow.expires_at { self.clear_pending_flow(&flow); return Err(MaenifoldError::Message("device code expired before authorization completed".to_string())); }
            match self.poll_device_token(&flow.device_code).await? {
                Ok(token) => {
                    self.clear_pending_flow(&flow);
                    return Ok(token);
                }
                Err(PollState::Pending) => tokio::time::sleep(poll_interval).await,
                Err(PollState::SlowDown) => {
                    poll_interval += Duration::from_secs(5);
                    tokio::time::sleep(poll_interval).await;
                }
                Err(PollState::Failed(message)) => {
                    self.clear_pending_flow(&flow);
                    return Err(MaenifoldError::Message(message));
                }
            }
        }
    }

    pub async fn refresh_token(&self, refresh_token: &str) -> Result<TokenInfo> {
        let url = self.token_endpoint();
        let (status, body) = self
            .post_form(
                &url,
                &[
                    ("client_id", self.client_id.as_str()),
                    ("grant_type", "refresh_token"),
                    ("refresh_token", refresh_token),
                ],
                "token refresh",
            )
            .await?;
        parse_token_info(&body, "token refresh response", status)
    }

    pub fn is_token_valid(token: &TokenInfo) -> bool {
        token.expires_at > Utc::now()
    }

    async fn poll_device_token(&self, device_code: &str) -> Result<std::result::Result<TokenInfo, PollState>> {
        let url = self.token_endpoint();
        let response = self
            .client
            .post(url)
            .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(urlencode_form(&[
                ("client_id", self.client_id.as_str()),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("device_code", device_code),
            ]))
            .send()
            .await
            .map_err(|error| MaenifoldError::Message(format!("device code token request failed: {error}")))?;
        let status = response.status();
        let body = response.text().await.map_err(|error| MaenifoldError::Message(format!("device code token response read failed: {error}")))?;
        if status.is_success() {
            return parse_token_info(&body, "device code token response", status).map(Ok);
        }
        let message = summarize_oauth_error(&body, status);
        let error = serde_json::from_str::<OAuthErrorResponse>(&body).ok();
        let state = match error.as_ref().map(|error| error.error.as_str()) {
            Some("authorization_pending") => PollState::Pending,
            Some("slow_down") => PollState::SlowDown,
            Some("expired_token") => PollState::Failed(format!("device code expired: {message}")),
            Some("access_denied") => PollState::Failed(format!("device code authorization denied: {message}")),
            Some(other) => PollState::Failed(format!("device code token request failed with {status}: {other}: {message}")),
            None => PollState::Failed(format!("device code token request failed with {status}: {message}")),
        };
        Ok(Err(state))
    }
    async fn post_form(&self, url: &str, form: &[(&str, &str)], context: &str) -> Result<(reqwest::StatusCode, String)> {
        let response = self
            .client
            .post(url)
            .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(urlencode_form(form))
            .send()
            .await
            .map_err(|error| MaenifoldError::Message(format!("{context} failed: {error}")))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| MaenifoldError::Message(format!("{context} response read failed: {error}")))?;
        if status.is_success() {
            Ok((status, body))
        } else {
            let details = summarize_oauth_error(&body, status);
            Err(MaenifoldError::Message(format!(
                "{context} failed with {status}: {details}"
            )))
        }
    }

    fn device_code_endpoint(&self) -> String { format!("https://login.microsoftonline.com/{}/oauth2/v2.0/devicecode", self.tenant_id) }

    fn token_endpoint(&self) -> String { format!("https://login.microsoftonline.com/{}/oauth2/v2.0/token", self.tenant_id) }

    fn pending_flow(&self, device_code: &str) -> Result<PendingDeviceFlow> {
        self.pending_flows
            .lock()
            .get(device_code)
            .cloned()
            .ok_or_else(|| {
                MaenifoldError::Message(
                    "device code not found; initiate the device code flow before acquiring a token"
                        .to_string(),
                )
            })
    }

    fn store_pending_flow(&self, flow: PendingDeviceFlow) {
        let mut pending = self.pending_flows.lock();
        pending.insert(flow.device_code.clone(), flow.clone());
        pending.insert(flow.user_code.clone(), flow);
    }

    fn clear_pending_flow(&self, flow: &PendingDeviceFlow) {
        let mut pending = self.pending_flows.lock();
        pending.remove(&flow.device_code);
        pending.remove(&flow.user_code);
    }
}
fn join_scopes(scopes: &[&str]) -> Result<String> {
    if scopes.is_empty() { Err(MaenifoldError::InvalidArgument("at least one Teams auth scope is required".to_string())) } else { Ok(scopes.join(" ")) }
}
fn parse_token_info(body: &str, context: &str, status: reqwest::StatusCode) -> Result<TokenInfo> {
    let response: TokenResponse = parse_json(body, context, status)?;
    Ok(TokenInfo {
        access_token: response.access_token,
        expires_at: Utc::now() + seconds_to_duration(response.expires_in),
        refresh_token: response.refresh_token,
    })
}
fn parse_json<T: for<'de> Deserialize<'de>>(body: &str, context: &str, status: reqwest::StatusCode) -> Result<T> {
    serde_json::from_str(body).map_err(|error| MaenifoldError::Message(format!("failed to parse {context} (status {status}): {error}")))
}
fn oauth_message(error: &OAuthErrorResponse) -> String {
    if error.error_description.trim().is_empty() { error.error.clone() } else { error.error_description.replace('\r', " ").replace('\n', " ") }
}
fn summarize_oauth_error(body: &str, status: reqwest::StatusCode) -> String {
    serde_json::from_str::<OAuthErrorResponse>(body)
        .map(|error| oauth_message(&error))
        .unwrap_or_else(|_| format!("unparseable error response (status {status})"))
}
fn seconds_to_duration(seconds: u64) -> ChronoDuration {
    ChronoDuration::seconds(seconds.min(i64::MAX as u64) as i64)
}
fn urlencode_form(form: &[(&str, &str)]) -> String {
    form.iter()
        .map(|(key, value)| {
            format!(
                "{}={}",
                urlencoding::encode(key),
                urlencoding::encode(value)
            )
        })
        .collect::<Vec<_>>()
        .join("&")
}
const fn default_poll_interval() -> u64 {
    5
}
