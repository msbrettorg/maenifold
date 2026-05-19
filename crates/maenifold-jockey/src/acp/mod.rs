mod adapter;
mod client;
mod client_fs;
mod client_terminal;
mod runtime_state;
mod session;
mod worker;

pub use adapter::{
    PREFLIGHT_ENV_VALIDATION_FAILED_PREFIX, StdioAdapterSpec, build_stdio_adapter,
    clear_adapter_cache, config_root, probe_runtime, resolve_cwd, set_config_root,
};
pub use runtime_state::{
    agent_capabilities, list_available_commands, list_discovered_config_options,
    list_discovered_models, list_discovered_modes,
};
pub use session::force_kill_session_sync;
pub use session::{
    cancel_session, cancel_tracked_session_blocking, execute_runtime, force_kill_session,
    list_live_sessions_blocking, prewarm_role, prewarm_role_for_config,
    prewarm_role_with_session_id, reconnect_session, refresh_role_config_defs, reset_session,
    set_config_option, set_mode,
};
pub use worker::{
    AcpEvent, AcpExecuteError, AcpExecuteResult, AcpExecuteSuccess, AcpPromptResult,
    ConnectionDeathEvent, PrewarmEvent, WorkerMsg, respond_to_permission, set_death_event_sender,
    set_prewarm_event_sender, shutdown, worker_tx,
};
