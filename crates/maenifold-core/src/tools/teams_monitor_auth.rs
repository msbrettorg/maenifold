use super::context::{ToolContext, render};
use super::teams_monitor::{load_monitor_config, start_monitor_supervisor, write_agents_config};
use super::teams_monitor_helpers::status_value;
use crate::acp_config::{
    AgentsConfig, MaenifoldSettings, TeamsMonitorConfig, TeamsMonitorIdentityConfig,
};
use crate::error::{MaenifoldError, Result};
use crate::teams::credential_store::{AGENCY_COWORK_TOKEN_RESOURCE, delete_agency_cowork_token};
use crate::teams::{
    TeamsIdentity, TeamsIdentitySource, TeamsTokenProvider,
    detect_identity as detect_teams_identity, set_teams_authenticated_until, set_teams_error,
    set_teams_unauthenticated, teams_runtime_snapshot,
};
use std::fmt::Display;
use std::future::Future;
use std::sync::LazyLock;
use tokio::runtime::{Builder, Handle, Runtime, RuntimeFlavor};
use tokio::task;

static TEAMS_AUTH_RUNTIME: LazyLock<std::result::Result<Runtime, String>> = LazyLock::new(|| {
    Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|error| format!("Failed to build Teams auth runtime: {error}"))
});

pub(super) fn detect_identity(context: &ToolContext) -> Result<String> {
    let root = &context.config.maenifold_root;
    let mut config = AgentsConfig::load(root)?;
    let mut settings = config
        .settings
        .clone()
        .unwrap_or_else(MaenifoldSettings::default);
    acquire_token_for_monitor(&mut settings.teams_monitor)?;
    let monitor = settings.teams_monitor.clone();
    config.settings = Some(settings);
    write_agents_config(root, &config)?;
    Ok(render(
        context.output_format,
        status_value(
            "detect_identity",
            &monitor,
            &teams_runtime_snapshot(),
            "Teams monitor identity detected from the signed-in Teams token.",
        ),
    ))
}

pub(super) fn refresh(context: &ToolContext) -> Result<String> {
    let root = &context.config.maenifold_root;
    let mut config = AgentsConfig::load(root)?;
    let mut settings = config
        .settings
        .clone()
        .unwrap_or_else(MaenifoldSettings::default);
    acquire_token_for_monitor(&mut settings.teams_monitor)?;
    config.settings = Some(settings);
    write_agents_config(root, &config)?;
    Ok(status_response(
        context,
        "refresh",
        "Teams token and identity refreshed from the signed-in environment.",
    )?)
}

pub(super) fn sign_out(context: &ToolContext) -> Result<String> {
    if let Err(error) = delete_agency_cowork_token(AGENCY_COWORK_TOKEN_RESOURCE) {
        set_teams_error(error.to_string());
        return Err(error);
    }
    set_teams_unauthenticated();
    Ok(status_response(
        context,
        "sign_out",
        "Teams token cache entry deleted; non-secret monitor configuration was preserved.",
    )?)
}

pub(super) fn restore_on_enable(monitor: &mut TeamsMonitorConfig) -> Result<()> {
    let token = acquire_token_for_monitor(monitor)?;
    let expires = monitor
        .identity
        .as_ref()
        .map(|id| id.expires_at.clone());
    start_monitor_supervisor(monitor, token)?;
    set_teams_authenticated_until(expires);
    if let Some(agent) = monitor.dispatch_agent.as_deref() {
        crate::teams::supervisor::prewarm_teams_session(agent);
    }
    Ok(())
}

pub(super) fn acquire_token_for_monitor(monitor: &mut TeamsMonitorConfig) -> Result<String> {
    let acquisition = run_teams_future(TeamsTokenProvider::new().acquire_token())?;
    let token = acquisition.token;
    let identity = run_teams_future(detect_teams_identity(&token.access_token))?;
    monitor.identity = Some(identity_config(&identity));
    if let Some(region) = identity.chatsvc_region.as_ref() {
        monitor.connection.chatsvc_region = region.clone();
    }
    set_teams_authenticated_until(Some(token.expires_at.to_rfc3339()));
    Ok(token.access_token)
}

fn identity_config(identity: &TeamsIdentity) -> TeamsMonitorIdentityConfig {
    TeamsMonitorIdentityConfig {
        mri: identity.mri.clone(),
        object_id: identity.object_id.clone(),
        tenant_id: identity.tenant_id.clone(),
        upn: identity.upn.clone(),
        display_name: identity.display_name.clone(),
        expires_at: identity.expires_at.to_rfc3339(),
        chatsvc_region: identity.chatsvc_region.clone(),
        source: match identity.identity_source {
            TeamsIdentitySource::Jwt => "jwt",
            TeamsIdentitySource::JwtWithGraphRegion => "jwtWithGraphRegion",
        }
        .to_string(),
    }
}

fn status_response(context: &ToolContext, action: &str, message: &str) -> Result<String> {
    let monitor = load_monitor_config(&context.config.maenifold_root)?;
    Ok(render(
        context.output_format,
        status_value(action, &monitor, &teams_runtime_snapshot(), message),
    ))
}

fn run_teams_future<F, T, E>(future: F) -> Result<T>
where
    F: Future<Output = std::result::Result<T, E>>,
    E: Display,
{
    let result = match Handle::try_current() {
        Ok(handle) => {
            if !matches!(handle.runtime_flavor(), RuntimeFlavor::MultiThread) {
                return Err(MaenifoldError::Message(
                    "Teams authentication requires a multi-thread Tokio runtime when invoked from async context"
                        .to_string(),
                ));
            }
            task::block_in_place(|| handle.block_on(future))
        }
        Err(_) => teams_auth_runtime()?.block_on(future),
    };
    result.map_err(|error| {
        let message = error.to_string();
        set_teams_error(&message);
        MaenifoldError::Message(message)
    })
}

fn teams_auth_runtime() -> Result<&'static Runtime> {
    TEAMS_AUTH_RUNTIME
        .as_ref()
        .map_err(|message| MaenifoldError::Message(message.clone()))
}
