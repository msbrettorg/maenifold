mod adapter;
mod client;
mod client_terminal;
mod runtime_state;
mod session;
mod worker;

pub use adapter::{
    StdioAdapterSpec, build_stdio_adapter, clear_adapter_cache, config_root, probe_runtime,
    resolve_cwd, set_config_root,
};
pub use runtime_state::{
    list_available_commands, list_discovered_config_options, list_discovered_models,
    list_discovered_modes,
};
pub use session::{
    cancel_session, execute_runtime, prewarm_role, prewarm_role_for_config,
    prewarm_role_with_session_id, reconnect_session, refresh_role_config_defs, reset_session,
    set_config_option, set_mode,
};
pub use worker::{
    AcpEvent, AcpPromptResult, ConnectionDeathEvent, PrewarmEvent, WorkerMsg,
    respond_to_permission, set_death_event_sender, set_prewarm_event_sender, shutdown, worker_tx,
};
