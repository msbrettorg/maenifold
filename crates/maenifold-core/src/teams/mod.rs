mod auth;
pub(crate) mod catchup;
mod chunking;
pub(crate) mod commands;
pub mod credential_store;
mod dedup;
mod dispatch;
pub(crate) mod dispatch_store;
pub(crate) mod dispatcher;
mod handler;
mod handler_helpers;
pub(crate) mod lifecycle;
mod prompt_guard;
mod prompt_guard_patterns;
pub(crate) mod credential_guard;
mod credential_guard_patterns;
pub(crate) mod region;
pub mod identity;
mod reply;
mod runtime;
pub mod supervisor;
mod token_refresh;
pub mod token_refresh_meta;
pub mod token_provider;
mod trouter;
mod trouter_handle;
pub(crate) mod trouter_protocol;
mod types;

pub use auth::TokenInfo;
pub use dedup::{DEFAULT_DEDUP_CAPACITY, MessageDedup};
pub use dispatch::{TeamsDispatchRequest, TeamsSessionMetadata};
pub use handler::{MatchResult, MessageHandler};
pub use handler_helpers::MessageHandlerConfig;
pub use identity::{TeamsIdentity, TeamsIdentitySource, TeamsJwtClaims, detect_identity};
pub use reply::{TeamsReplyRequest, reply_payload, reply_url, send_reply};
pub use runtime::{
    TEAMS_STATUS_URI, TeamsPrompt, TeamsRuntimeState,
    clear_chatsvc_token, current_chatsvc_token, push_teams_prompt, set_chatsvc_token,
    set_teams_auth_expired, set_teams_authenticated, set_teams_authenticated_until,
    set_teams_connected, set_teams_connecting, set_teams_disabled, set_teams_error,
    set_teams_pending_auth, set_teams_unauthenticated, subscribe_teams_prompts,
    teams_runtime_snapshot, teams_runtime_state,
};
pub use token_provider::{
    TEAMS_CHATSVC_TOKEN_ENV, TEAMS_TOKEN_RESOURCE, TeamsTokenAcquisition, TeamsTokenError,
    TeamsTokenProvider, TeamsTokenSource,
};
pub use trouter::{TrouterClient, TrouterClientConfig};
pub use trouter_handle::ReRegisterHandle;
pub use trouter_protocol::parse_frame;
pub use types::{EventMessage, TrouterFrame};
