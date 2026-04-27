use std::sync::{Arc, LazyLock, OnceLock};

use parking_lot::RwLock;
use serde::Serialize;
use tokio::runtime::{Builder, Handle, Runtime};

use crate::mcp::resources::notify_resource_updated;

pub const TEAMS_STATUS_URI: &str = "teams://status";

static NOTIFICATION_RUNTIME: LazyLock<std::result::Result<Runtime, String>> = LazyLock::new(|| {
    Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("Failed to build Teams notification runtime: {error}"))
});

static TEAMS_RUNTIME_STATE: LazyLock<Arc<RwLock<TeamsRuntimeState>>> =
    LazyLock::new(|| Arc::new(RwLock::new(TeamsRuntimeState::default())));

static TEAMS_PROMPT_TX: OnceLock<tokio::sync::broadcast::Sender<TeamsPrompt>> = OnceLock::new();

/// Current chatsvcagg bearer token, refreshed by the supervisor's token-refresh loop.
/// Port of agency-cowork's module-level `set_chatsvc_token` in
/// `skills/teams/scripts/monitor/service.py` (referenced by `_token_refresh_loop` L719-L760).
static CHATSVC_TOKEN: LazyLock<RwLock<Option<String>>> =
    LazyLock::new(|| RwLock::new(None));

pub fn set_chatsvc_token(token: String) {
    *CHATSVC_TOKEN.write() = Some(token);
}

pub fn current_chatsvc_token() -> Option<String> {
    CHATSVC_TOKEN.read().clone()
}

pub fn clear_chatsvc_token() {
    *CHATSVC_TOKEN.write() = None;
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TeamsRuntimeState {
    #[serde(rename = "connectionState")]
    pub connection_state: String,
    pub auth_state: String,
    pub expires_at: Option<String>,
    pub last_auth_error: Option<String>,
    pub last_message_at: Option<String>,
    pub last_match_at: Option<String>,
    pub match_count: u32,
    pub active_dispatches: u32,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TeamsPrompt {
    pub prompt: String,
    pub conversation_id: String,
    pub sender_name: String,
    pub matched_keyword: String,
}

impl Default for TeamsRuntimeState {
    fn default() -> Self {
        Self {
            connection_state: "disabled".to_string(),
            auth_state: "unauthenticated".to_string(),
            expires_at: None,
            last_auth_error: None,
            last_message_at: None,
            last_match_at: None,
            match_count: 0,
            active_dispatches: 0,
            last_error: None,
        }
    }
}

impl TeamsRuntimeState {
    pub fn enabled(&self) -> bool {
        self.connection_state != "disabled"
    }

    pub fn mark_auth_pending(&mut self) {
        self.auth_state = "pending".to_string();
        self.expires_at = None;
        self.last_auth_error = None;
    }

    pub fn mark_authenticated(&mut self, expires_at: Option<String>) {
        self.auth_state = "authenticated".to_string();
        self.expires_at = expires_at;
        self.last_auth_error = None;
    }

    pub fn mark_unauthenticated(&mut self) {
        self.auth_state = "unauthenticated".to_string();
        self.expires_at = None;
        self.last_auth_error = None;
    }

    pub fn mark_auth_expired(&mut self, message: impl AsRef<str>) {
        self.auth_state = "expired".to_string();
        self.expires_at = None;
        self.last_auth_error = Some(sanitize_auth_error(message.as_ref()));
    }

    pub fn mark_auth_error(&mut self, message: impl AsRef<str>) {
        self.auth_state = "error".to_string();
        self.expires_at = None;
        self.last_auth_error = Some(sanitize_auth_error(message.as_ref()));
    }

    pub fn status_expires_at(&self) -> Option<&str> {
        (self.auth_state == "authenticated")
            .then_some(self.expires_at.as_deref())
            .flatten()
    }
}

pub fn teams_runtime_state() -> Arc<RwLock<TeamsRuntimeState>> {
    TEAMS_RUNTIME_STATE.clone()
}

pub fn teams_runtime_snapshot() -> TeamsRuntimeState {
    teams_runtime_state().read().clone()
}

pub fn set_teams_connecting() {
    update_teams_runtime_state(|state| {
        state.connection_state = "connecting".to_string();
        state.last_error = None;
    });
}

pub fn set_teams_connected() {
    update_teams_runtime_state(|state| {
        state.connection_state = "connected".to_string();
        state.last_error = None;
    });
}

pub fn set_teams_pending_auth() {
    update_teams_runtime_state(|state| {
        state.connection_state = "pending_auth".to_string();
        state.mark_auth_pending();
        state.last_error = None;
    });
}

pub fn set_teams_authenticated() {
    set_teams_authenticated_until(None);
}

pub fn set_teams_authenticated_until(expires_at: Option<String>) {
    update_teams_runtime_state(|state| {
        state.mark_authenticated(expires_at);
        state.last_error = None;
    });
}

pub fn set_teams_unauthenticated() {
    update_teams_runtime_state(|state| {
        state.connection_state = "unauthenticated".to_string();
        state.mark_unauthenticated();
        state.active_dispatches = 0;
        state.last_error = None;
    });
}

pub fn set_teams_disabled() {
    update_teams_runtime_state(|state| {
        state.connection_state = "disabled".to_string();
        state.mark_unauthenticated();
        state.active_dispatches = 0;
        state.last_error = None;
    });
}

pub fn set_teams_auth_expired(message: impl Into<String>) {
    update_teams_runtime_state(|state| {
        state.connection_state = "unauthenticated".to_string();
        state.mark_auth_expired(message.into());
        state.active_dispatches = 0;
    });
}

pub fn set_teams_error(message: impl Into<String>) {
    update_teams_runtime_state(|state| {
        let message = message.into();
        state.connection_state = "error".to_string();
        state.mark_auth_error(&message);
        state.last_error = Some(message);
        state.active_dispatches = 0;
    });
}

pub fn push_teams_prompt(prompt: TeamsPrompt) {
    let tx = TEAMS_PROMPT_TX.get_or_init(|| {
        let (tx, _) = tokio::sync::broadcast::channel(64);
        tx
    });
    let _ = tx.send(prompt);
    update_teams_runtime_state(|state| {
        state.match_count = state.match_count.saturating_add(1);
        state.last_match_at = Some(chrono::Utc::now().to_rfc3339());
    });
}

pub fn subscribe_teams_prompts() -> tokio::sync::broadcast::Receiver<TeamsPrompt> {
    TEAMS_PROMPT_TX
        .get_or_init(|| {
            let (tx, _) = tokio::sync::broadcast::channel(64);
            tx
        })
        .subscribe()
}

pub fn update_teams_runtime_state(update: impl FnOnce(&mut TeamsRuntimeState)) {
    let changed = {
        let lock = teams_runtime_state();
        let mut guard = lock.write();
        let before = guard.clone();
        update(&mut guard);
        *guard != before
    };

    if changed {
        emit_teams_status_updated();
    }
}

fn emit_teams_status_updated() {
    if let Ok(handle) = Handle::try_current() {
        handle.spawn(async {
            notify_resource_updated(TEAMS_STATUS_URI).await;
        });
        return;
    }

    if let Ok(runtime) = NOTIFICATION_RUNTIME.as_ref() {
        runtime.block_on(async {
            notify_resource_updated(TEAMS_STATUS_URI).await;
        });
    }
}

fn sanitize_auth_error(message: &str) -> String {
    let lower = message.to_ascii_lowercase();
    if lower.contains("expired") {
        "expired".to_string()
    } else if lower.contains("authorization_pending") || lower.contains("pending") {
        "authorization_pending".to_string()
    } else if lower.contains("access_denied") || lower.contains("denied") {
        "access_denied".to_string()
    } else if lower.contains("credential") || lower.contains("keyring") {
        "credential_store_error".to_string()
    } else if lower.contains("runtime") || lower.contains("tokio") {
        "runtime_error".to_string()
    } else if lower.contains("timeout")
        || lower.contains("connect")
        || lower.contains("http")
        || lower.contains("network")
        || lower.contains("request")
    {
        "network_error".to_string()
    } else if lower.contains("json") || lower.contains("parse") {
        "response_parse_error".to_string()
    } else if lower.contains("auth") || lower.contains("oauth") || lower.contains("token") {
        "authentication_error".to_string()
    } else {
        "unknown_error".to_string()
    }
}
