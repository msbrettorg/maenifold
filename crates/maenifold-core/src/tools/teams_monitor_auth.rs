use super::context::{ToolContext, render, required_string};
use super::teams_monitor::{load_monitor_config, optional_string_vec, start_monitor_supervisor, write_agents_config};
use super::teams_monitor_helpers::status_value;
use crate::acp_config::{AgentsConfig, MaenifoldSettings, TeamsMonitorConfig};
use crate::error::{MaenifoldError, Result};
use crate::teams::credential_store::{delete_token, retrieve_token, store_token};
use crate::teams::{
    TeamsAuth, TokenInfo, set_teams_auth_expired, set_teams_authenticated_until,
    set_teams_error, set_teams_pending_auth, set_teams_unauthenticated, teams_runtime_snapshot,
};
use parking_lot::RwLock;
use serde_json::{Value, json};
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock};
use tokio::runtime::{Builder, Handle, Runtime, RuntimeFlavor};
use tokio::task;

static TEAMS_AUTH_RUNTIME: LazyLock<std::result::Result<Runtime, String>> = LazyLock::new(|| {
    Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|error| format!("Failed to build Teams auth runtime: {error}"))
});

static PENDING_DEVICE_CODE: LazyLock<RwLock<Option<PendingDeviceCode>>> =
    LazyLock::new(|| RwLock::new(None));
static AUTH_GENERATION: AtomicU64 = AtomicU64::new(0);

const DEFAULT_TEAMS_SCOPES: &[&str] = &["https://api.spaces.skype.com/.default"];

struct PendingDeviceCode {
    auth: Arc<TeamsAuth>,
    client_id: String,
    tenant_id: String,
    user_code: String,
    generation: u64,
}

pub(super) fn authenticate_start(context: &ToolContext, payload: &Value) -> Result<String> {
    let client_id = required_string(payload, "clientId")?.to_string();
    let tenant_id = required_string(payload, "tenantId")?.to_string();
    let scopes = validated_scopes(payload)?;

    let mut pending = PENDING_DEVICE_CODE.write();
    if pending.is_some() {
        return Err(MaenifoldError::InvalidArgument(
            "Teams device-code authentication is already pending; complete it before starting another flow"
                .to_string(),
        ));
    }
    persist_auth_ids(context, &client_id, &tenant_id)?;

    let scope_refs = scopes.iter().map(String::as_str).collect::<Vec<_>>();
    let auth = Arc::new(TeamsAuth::new(client_id.clone(), tenant_id.clone()));
    let generation = AUTH_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let info = run_teams_future(auth.initiate_device_code(&scope_refs))
        .map_err(MaenifoldError::Message)?;
    *pending = Some(PendingDeviceCode {
        auth,
        client_id: client_id.clone(),
        tenant_id: tenant_id.clone(),
        user_code: info.user_code.clone(),
        generation,
    });
    set_teams_pending_auth();

    Ok(render(
        context.output_format,
        json!({
            "action": "authenticate_start",
            "clientId": client_id,
            "tenantId": tenant_id,
            "scopes": scopes,
            "userCode": info.user_code,
            "verificationUri": info.verification_uri,
            "expiresIn": info.expires_in,
            "message": info.message,
        }),
    ))
}

pub(super) fn authenticate_complete(context: &ToolContext) -> Result<String> {
    let pending = PENDING_DEVICE_CODE.write().take().ok_or_else(|| {
        MaenifoldError::InvalidArgument("no Teams authentication is pending".to_string())
    })?;

    let token = run_teams_future(pending.auth.acquire_token(&pending.user_code))
        .map_err(MaenifoldError::Message);
    let token = match token { Ok(token) => token, Err(error) => {
        set_teams_error(error.to_string());
        return Err(error);
    } };
    let _auth_guard = PENDING_DEVICE_CODE.write();
    if !pending_can_store_token(&pending) {
        return Ok(status_response(
            context,
            "authenticate_complete",
            "Teams authentication completed after sign_out; credentials were not stored.",
        )?);
    }
    if let Err(error) = store_token(&pending.client_id, &pending.tenant_id, &token) {
        set_teams_error(error.to_string()); return Err(error);
    }
    set_teams_authenticated_until(Some(token.expires_at.to_rfc3339()));

    Ok(status_response(
        context,
        "authenticate_complete",
        "Teams authentication completed and credentials were stored securely.",
    )?)
}

pub(super) fn refresh(context: &ToolContext) -> Result<String> {
    let monitor = load_monitor_config(&context.config.maenifold_root)?;
    let (client_id, tenant_id) = auth_ids(&monitor)?;
    let token = match retrieve_token(&client_id, &tenant_id)? {
        Some(token) => token,
        None => {
            set_teams_unauthenticated();
            return Ok(status_response(
                context,
                "refresh",
                "No Teams credentials are stored; authenticate before refreshing.",
            )?);
        }
    };

    match refresh_if_needed(&client_id, &tenant_id, token) {
        Ok(token) => set_teams_authenticated_until(Some(token.expires_at.to_rfc3339())),
        Err(error) => {
            delete_token(&client_id, &tenant_id)?;
            record_auth_failure(&error);
            return Err(error);
        }
    }

    Ok(status_response(
        context,
        "refresh",
        "Teams credentials are valid.",
    )?)
}

pub(super) fn sign_out(context: &ToolContext) -> Result<String> {
    let monitor = load_monitor_config(&context.config.maenifold_root)?;
    clear_all_pending_auth();
    if let Ok((client_id, tenant_id)) = auth_ids(&monitor) {
        delete_token(&client_id, &tenant_id)?;
    }
    set_teams_unauthenticated();
    Ok(status_response(
        context,
        "sign_out",
        "Teams credentials deleted and monitor signed out.",
    )?)
}

pub(super) fn restore_on_enable(monitor: &TeamsMonitorConfig) -> Result<()> {
    let (client_id, tenant_id) = match auth_ids(monitor) {
        Ok(ids) => ids,
        Err(_) => {
            set_teams_unauthenticated();
            return Err(MaenifoldError::Message("authentication required — run authenticate_start first".to_string()));
        }
    };
    let Some(token) = retrieve_token(&client_id, &tenant_id)? else {
        set_teams_unauthenticated();
        return Err(MaenifoldError::Message("authentication required — run authenticate_start first".to_string()));
    };
    match refresh_if_needed(&client_id, &tenant_id, token) {
        Ok(token) => {
            set_teams_authenticated_until(Some(token.expires_at.to_rfc3339()));
            start_monitor_supervisor(monitor, token.access_token)?;
            Ok(())
        }
        Err(error) => { delete_token(&client_id, &tenant_id)?; record_auth_failure(&error); Err(error) }
    }
}

fn persist_auth_ids(context: &ToolContext, client_id: &str, tenant_id: &str) -> Result<()> {
    let root = &context.config.maenifold_root;
    let mut config = AgentsConfig::load(root)?;
    let mut settings = config
        .settings
        .clone()
        .unwrap_or_else(MaenifoldSettings::default);
    settings.teams_monitor.client_id = Some(client_id.to_string());
    settings.teams_monitor.tenant_id = Some(tenant_id.to_string());
    config.settings = Some(settings);
    write_agents_config(root, &config)
}

fn refresh_if_needed(client_id: &str, tenant_id: &str, token: TokenInfo) -> Result<TokenInfo> {
    if TeamsAuth::is_token_valid(&token) {
        return Ok(token);
    }
    let Some(refresh_token) = token.refresh_token.as_deref() else {
        return Err(MaenifoldError::Message(
            "Teams token is expired and no refresh token is available".to_string(),
        ));
    };
    let auth = TeamsAuth::new(client_id.to_string(), tenant_id.to_string());
    let refreshed =
        run_teams_future(auth.refresh_token(refresh_token)).map_err(MaenifoldError::Message)?;
    store_token(client_id, tenant_id, &refreshed)?;
    Ok(refreshed)
}

fn record_auth_failure(error: &MaenifoldError) {
    let message = error.to_string();
    if message.to_ascii_lowercase().contains("expired") {
        set_teams_auth_expired(message);
    } else { set_teams_error(message); }
}

fn validated_scopes(payload: &Value) -> Result<Vec<String>> {
    let scopes = optional_string_vec(payload, "scopes")?.unwrap_or_else(|| {
        DEFAULT_TEAMS_SCOPES
            .iter()
            .map(|scope| (*scope).to_string())
            .collect()
    });
    if scopes.is_empty()
        || scopes
            .iter()
            .any(|scope| !DEFAULT_TEAMS_SCOPES.contains(&scope.as_str()))
    {
        return Err(MaenifoldError::InvalidArgument(format!(
            "'scopes' must be omitted or limited to: {}",
            DEFAULT_TEAMS_SCOPES.join(", ")
        )));
    }
    Ok(scopes)
}

fn auth_ids(monitor: &TeamsMonitorConfig) -> Result<(String, String)> {
    let client_id = monitor.client_id.clone().ok_or_else(|| {
        MaenifoldError::InvalidArgument(
            "Teams monitor has no persisted clientId; run authenticate first".to_string(),
        )
    })?;
    let tenant_id = monitor.tenant_id.clone().ok_or_else(|| {
        MaenifoldError::InvalidArgument(
            "Teams monitor has no persisted tenantId; run authenticate first".to_string(),
        )
    })?;
    Ok((client_id, tenant_id))
}

fn status_response(context: &ToolContext, action: &str, message: &str) -> Result<String> {
    let monitor = load_monitor_config(&context.config.maenifold_root)?;
    Ok(render(
        context.output_format,
        status_value(action, &monitor, &teams_runtime_snapshot(), message),
    ))
}

fn clear_all_pending_auth() {
    let mut pending = PENDING_DEVICE_CODE.write();
    AUTH_GENERATION.fetch_add(1, Ordering::SeqCst);
    *pending = None;
}

fn pending_can_store_token(pending: &PendingDeviceCode) -> bool {
    if pending.generation != AUTH_GENERATION.load(Ordering::SeqCst) {
        return false;
    }
    matches!(
        teams_runtime_snapshot().connection_state.as_str(),
        "pending_auth" | "authenticated"
    )
}

fn run_teams_future<F, T>(future: F) -> std::result::Result<T, String>
where
    F: Future<Output = Result<T>>,
{
    match Handle::try_current() {
        Ok(handle) => {
            if !matches!(handle.runtime_flavor(), RuntimeFlavor::MultiThread) {
                return Err(
                    "Teams authentication requires a multi-thread Tokio runtime when invoked from async context"
                        .to_string(),
                );
            }
            task::block_in_place(|| handle.block_on(future)).map_err(|error| error.to_string())
        }
        Err(_) => teams_auth_runtime()?
            .block_on(future)
            .map_err(|error| error.to_string()),
    }
}

fn teams_auth_runtime() -> std::result::Result<&'static Runtime, String> {
    TEAMS_AUTH_RUNTIME
        .as_ref()
        .map_err(|message| message.clone())
}
