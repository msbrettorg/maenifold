mod auth;
pub mod credential_store;
mod dispatch;
mod handler;
mod runtime;
pub mod supervisor;
mod trouter;
mod types;

pub use auth::{DeviceCodeInfo, TeamsAuth, TokenInfo};
pub use dispatch::{TeamsDispatchRequest, TeamsSessionMetadata};
pub use handler::{MatchResult, MessageHandler, MessageHandlerConfig};
pub use runtime::{
    TEAMS_STATUS_URI, TeamsRuntimeState, set_teams_auth_expired, set_teams_authenticated,
    set_teams_authenticated_until, set_teams_connecting, set_teams_disabled, set_teams_error,
    set_teams_pending_auth, set_teams_unauthenticated, teams_runtime_snapshot, teams_runtime_state,
};
pub use trouter::{TrouterClient, parse_frame};
pub use types::{EventMessage, TrouterFrame};
