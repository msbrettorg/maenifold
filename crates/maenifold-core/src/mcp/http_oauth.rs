use crate::acp_config::HttpAuthConfig;
use crate::error::{MaenifoldError, Result};
use reqwest::Url;
use rmcp::transport::{
    auth::{
        AuthClient, AuthError, AuthorizationManager, AuthorizationMetadata, ClientCredentialsConfig,
    },
    streamable_http_client::StreamableHttpError,
};

use super::gateway_session::HttpServerConfig;
use super::http_oauth_client::{GatewayHttpClient, OAuthHttpClient};

pub(super) struct PreparedHttpClient {
    pub(super) client: GatewayHttpClient,
    pub(super) auth_header: Option<String>,
}

impl OAuthHttpClient {
    pub(super) async fn new(
        config: &HttpServerConfig,
        http_client: reqwest::Client,
    ) -> Result<Self> {
        let mut manager = AuthorizationManager::new(config.url.as_str())
            .await
            .map_err(map_auth_error)?;
        manager
            .with_client(http_client.clone())
            .map_err(map_auth_error)?;

        let mut metadata = manager.discover_metadata().await.map_err(map_auth_error)?;
        apply_authority_overrides(&mut metadata, config.auth.as_ref())?;
        manager.set_metadata(metadata);

        let client_credentials =
            build_client_credentials(config.auth.as_ref(), config.url.as_str(), &manager)?;
        manager
            .validate_client_credentials_metadata(&client_credentials)
            .map_err(map_auth_error)?;
        manager
            .configure_client_credentials(&client_credentials)
            .map_err(map_auth_error)?;
        manager
            .exchange_client_credentials(&client_credentials)
            .await
            .map_err(map_auth_error)?;

        Ok(Self {
            inner: AuthClient::new(http_client, manager),
            client_credentials,
        })
    }
}

pub(super) async fn prepare_http_client(
    config: &HttpServerConfig,
    http_client: reqwest::Client,
) -> Result<PreparedHttpClient> {
    if config.auth.is_none() {
        return Ok(PreparedHttpClient {
            client: GatewayHttpClient::Plain(http_client),
            auth_header: None,
        });
    }

    let oauth_client = OAuthHttpClient::new(config, http_client).await?;
    let auth_header = Some(
        oauth_client
            .current_token()
            .await
            .map_err(map_transport_error)?,
    );

    Ok(PreparedHttpClient {
        client: GatewayHttpClient::OAuth(oauth_client),
        auth_header,
    })
}

fn build_client_credentials(
    auth: Option<&HttpAuthConfig>,
    base_url: &str,
    manager: &AuthorizationManager,
) -> Result<ClientCredentialsConfig> {
    let auth =
        auth.ok_or_else(|| MaenifoldError::Message("missing HTTP auth config".to_string()))?;
    let scopes = if auth.scopes.is_empty() {
        manager.select_scopes(None, &[])
    } else {
        auth.scopes.clone()
    };

    if auth.client_certificate_path.is_some() && auth.client_secret.is_none() {
        return Err(MaenifoldError::Message(
            "HTTP MCP OAuth clientCertificatePath is not supported without rmcp JWT auth features"
                .to_string(),
        ));
    }

    let client_secret = auth.client_secret.clone().ok_or_else(|| {
        MaenifoldError::Message(
            "HTTP MCP OAuth requires clientSecret until certificate auth is enabled".to_string(),
        )
    })?;

    Ok(ClientCredentialsConfig::ClientSecret {
        client_id: auth.client_id.clone(),
        client_secret,
        scopes,
        resource: Some(match auth.resource.clone() {
            Some(resource) => resource,
            None => oauth_resource(base_url)?,
        }),
    })
}

fn oauth_resource(base_url: &str) -> Result<String> {
    let url = Url::parse(base_url).map_err(|error| {
        MaenifoldError::Message(format!("invalid HTTP MCP URL '{base_url}': {error}"))
    })?;
    Ok(url.origin().ascii_serialization())
}

fn apply_authority_overrides(
    metadata: &mut AuthorizationMetadata,
    auth: Option<&HttpAuthConfig>,
) -> Result<()> {
    let Some(tenant_id) = auth.and_then(|auth| auth.tenant_id.as_deref()) else {
        return Ok(());
    };

    metadata.authorization_endpoint =
        rewrite_aad_tenant(&metadata.authorization_endpoint, tenant_id)?;
    metadata.token_endpoint = rewrite_aad_tenant(&metadata.token_endpoint, tenant_id)?;
    metadata.registration_endpoint = metadata
        .registration_endpoint
        .as_deref()
        .map(|value| rewrite_aad_tenant(value, tenant_id))
        .transpose()?;
    metadata.issuer = metadata
        .issuer
        .as_deref()
        .map(|value| rewrite_aad_tenant(value, tenant_id))
        .transpose()?;
    Ok(())
}

fn rewrite_aad_tenant(endpoint: &str, tenant_id: &str) -> Result<String> {
    let mut url = Url::parse(endpoint).map_err(|error| {
        MaenifoldError::Message(format!("invalid OAuth endpoint '{endpoint}': {error}"))
    })?;
    let Some(host) = url.host_str() else {
        return Ok(endpoint.to_string());
    };
    if !matches!(host, "login.microsoftonline.com" | "login.windows.net") {
        return Ok(endpoint.to_string());
    }

    let Some(segments) = url.path_segments() else {
        return Ok(endpoint.to_string());
    };
    let mut segments: Vec<String> = segments.map(str::to_string).collect();
    if segments
        .first()
        .is_none_or(|segment| !matches!(segment.as_str(), "common" | "organizations" | "consumers"))
    {
        return Ok(endpoint.to_string());
    }
    segments[0] = tenant_id.to_string();
    url.set_path(&segments.join("/"));
    Ok(url.to_string())
}

fn map_auth_error(error: AuthError) -> MaenifoldError {
    MaenifoldError::Message(format!("HTTP MCP OAuth setup failed: {error}"))
}

fn map_transport_error(error: StreamableHttpError<reqwest::Error>) -> MaenifoldError {
    MaenifoldError::Message(format!("HTTP MCP OAuth setup failed: {error}"))
}
