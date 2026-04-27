mod auth;
pub mod credential_store;
mod dispatch;
mod handler;
pub mod identity;
mod reply;
mod runtime;
pub mod supervisor;
pub mod token_provider;
mod trouter;
mod trouter_protocol;
mod types;

pub use auth::TokenInfo;
pub use dispatch::{TeamsDispatchRequest, TeamsSessionMetadata};
pub use handler::{MatchResult, MessageHandler, MessageHandlerConfig};
pub use identity::{TeamsIdentity, TeamsIdentitySource, TeamsJwtClaims, detect_identity};
pub use reply::{TeamsReplyRequest, reply_payload, reply_url, send_reply};
pub use runtime::{
    TEAMS_STATUS_URI, TeamsPrompt, TeamsRuntimeState,
    push_teams_prompt, set_teams_auth_expired, set_teams_authenticated,
    set_teams_authenticated_until, set_teams_connected, set_teams_connecting, set_teams_disabled,
    set_teams_error, set_teams_pending_auth, set_teams_unauthenticated,
    subscribe_teams_prompts, teams_runtime_snapshot, teams_runtime_state,
};
pub use token_provider::{
    TEAMS_CHATSVC_TOKEN_ENV, TEAMS_TOKEN_RESOURCE, TeamsTokenAcquisition, TeamsTokenError,
    TeamsTokenProvider, TeamsTokenSource,
};
pub use trouter::{TrouterClient, TrouterClientConfig};
pub use trouter_protocol::parse_frame;
pub use types::{EventMessage, TrouterFrame};
