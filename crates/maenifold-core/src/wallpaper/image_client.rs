use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use base64::Engine as _;
use reqwest::header::RETRY_AFTER;
use serde::{Deserialize, Serialize};

use crate::acp_config::WallpaperSettings;

const API_VERSION: &str = "2025-04-01-preview";

pub struct ImageClient {
    endpoint: String,
    api_key: String,
    image_model: String,
    max_retries: u32,
    http: reqwest::Client,
}

impl ImageClient {
    pub fn new(
        endpoint: impl Into<String>,
        api_key: impl Into<String>,
        image_model: impl Into<String>,
        max_retries: u32,
    ) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .context("failed to build Azure image HTTP client")?;

        Ok(Self {
            endpoint: endpoint.into(),
            api_key: api_key.into(),
            image_model: image_model.into(),
            max_retries,
            http,
        })
    }

    pub fn from_config(settings: &WallpaperSettings) -> Result<Self> {
        let endpoint = std::env::var(&settings.azure.endpoint_env_var).with_context(|| {
            format!(
                "missing Azure OpenAI endpoint env var {}",
                settings.azure.endpoint_env_var
            )
        })?;
        let api_key = std::env::var(&settings.azure.api_key_env_var).with_context(|| {
            format!(
                "missing Azure OpenAI API key env var {}",
                settings.azure.api_key_env_var
            )
        })?;

        Self::new(
            endpoint,
            api_key,
            settings.azure.image_model.clone(),
            settings.max_retries,
        )
    }

    pub async fn generate(&self, prompt: &str, size: &str, quality: &str) -> Result<Vec<u8>> {
        let url = self.generations_url();
        let body = GenerationRequest {
            prompt,
            size,
            quality,
            n: 1,
        };

        let response = self
            .send_with_retry(|| {
                Ok(self
                    .http
                    .post(&url)
                    .header("api-key", &self.api_key)
                    .json(&body))
            })
            .await?;
        decode_image_response(response)
    }

    pub async fn edit(
        &self,
        prompt: &str,
        previous_image: &[u8],
        size: &str,
        quality: &str,
        input_fidelity: &str,
    ) -> Result<Vec<u8>> {
        let url = self.edits_url();

        let response = self
            .send_with_retry(|| {
                let image_part = reqwest::multipart::Part::bytes(previous_image.to_vec())
                    .file_name("previous.png")
                    .mime_str("image/png")
                    .context("failed to build wallpaper edit image part")?;
                let form = reqwest::multipart::Form::new()
                    .part("image", image_part)
                    .text("prompt", prompt.to_string())
                    .text("size", size.to_string())
                    .text("quality", quality.to_string())
                    .text("input_fidelity", input_fidelity.to_string());

                Ok(self
                    .http
                    .post(&url)
                    .header("api-key", &self.api_key)
                    .multipart(form))
            })
            .await?;
        decode_image_response(response)
    }

    async fn send_with_retry<F>(&self, mut build_request: F) -> Result<ImageResponse>
    where
        F: FnMut() -> Result<reqwest::RequestBuilder>,
    {
        let mut attempt = 0;
        loop {
            let response = build_request()?
                .send()
                .await
                .context("Azure image request failed")?;
            let status = response.status();

            if status.is_success() {
                return response
                    .json::<ImageResponse>()
                    .await
                    .context("failed to parse Azure image response");
            }

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS && attempt < self.max_retries {
                let delay = retry_after_delay(response.headers())
                    .unwrap_or_else(|| fallback_retry_delay(attempt));
                tokio::time::sleep(delay).await;
                attempt += 1;
                continue;
            }

            let body = response
                .text()
                .await
                .context("failed to read Azure image error response")?;
            return Err(azure_error(status, &body));
        }
    }

    fn generations_url(&self) -> String {
        self.operation_url("generations")
    }

    fn edits_url(&self) -> String {
        self.operation_url("edits")
    }

    fn operation_url(&self, operation: &str) -> String {
        format!(
            "{}/openai/deployments/{}/images/{}?api-version={}",
            self.endpoint.trim_end_matches('/'),
            urlencoding::encode(&self.image_model),
            operation,
            API_VERSION
        )
    }
}

#[derive(Serialize)]
struct GenerationRequest<'a> {
    prompt: &'a str,
    size: &'a str,
    quality: &'a str,
    n: u32,
}

#[derive(Deserialize)]
struct ImageResponse {
    data: Vec<ImageData>,
}

#[derive(Deserialize)]
struct ImageData {
    b64_json: Option<String>,
}

#[derive(Deserialize)]
struct AzureErrorResponse {
    error: AzureError,
}

#[derive(Deserialize)]
struct AzureError {
    code: Option<String>,
    message: Option<String>,
}

fn decode_image_response(response: ImageResponse) -> Result<Vec<u8>> {
    let b64_json = response
        .data
        .first()
        .and_then(|data| data.b64_json.as_deref())
        .ok_or_else(|| anyhow!("Azure image response missing data[0].b64_json"))?;

    base64::engine::general_purpose::STANDARD
        .decode(b64_json)
        .context("failed to decode Azure image b64_json")
}

fn azure_error(status: reqwest::StatusCode, body: &str) -> anyhow::Error {
    match serde_json::from_str::<AzureErrorResponse>(body) {
        Ok(error) => {
            let code = error.error.code.unwrap_or_else(|| "unknown".to_string());
            let message = error
                .error
                .message
                .unwrap_or_else(|| "no error message returned".to_string());
            anyhow!("Azure image request failed with {status}: {code}: {message}")
        }
        Err(parse_error) => anyhow!(
            "Azure image request failed with {status}; error body was not Azure JSON ({parse_error})"
        ),
    }
}

fn retry_after_delay(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    headers
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs)
}

fn fallback_retry_delay(attempt: u32) -> Duration {
    let seconds = 2_u64.saturating_pow(attempt.min(5));
    Duration::from_secs(seconds.max(1))
}
