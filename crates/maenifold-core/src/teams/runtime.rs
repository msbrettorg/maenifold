use std::sync::{Arc, LazyLock};

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

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TeamsRuntimeState {
    #[serde(rename = "connectionState")]
    pub connection_state: String,
    pub last_message_at: Option<String>,
    pub last_match_at: Option<String>,
    pub match_count: u32,
    pub active_dispatches: u32,
    pub last_error: Option<String>,
}

impl Default for TeamsRuntimeState {
    fn default() -> Self {
        Self {
            connection_state: "disabled".to_string(),
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

pub fn set_teams_pending_auth() {
    update_teams_runtime_state(|state| {
        state.connection_state = "pending_auth".to_string();
        state.last_error = None;
    });
}

pub fn set_teams_authenticated() {
    update_teams_runtime_state(|state| {
        state.connection_state = "authenticated".to_string();
        state.last_error = None;
    });
}

pub fn set_teams_unauthenticated() {
    update_teams_runtime_state(|state| {
        state.connection_state = "unauthenticated".to_string();
        state.active_dispatches = 0;
        state.last_error = None;
    });
}

pub fn set_teams_disabled() {
    update_teams_runtime_state(|state| {
        state.connection_state = "disabled".to_string();
        state.active_dispatches = 0;
        state.last_error = None;
    });
}

pub fn set_teams_error(message: impl Into<String>) {
    update_teams_runtime_state(|state| {
        state.connection_state = "error".to_string();
        state.last_error = Some(message.into());
        state.active_dispatches = 0;
    });
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
