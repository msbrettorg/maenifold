//! Cloneable Trouter re-registration handle.
//!
//! Used by the token-refresh loop (T-238.50) to push a fresh chatsvcagg token
//! into the registrar without owning `&mut TrouterClient`. Mirrors the
//! `client.register()` call inside agency-cowork's `_token_refresh_loop`
//! (`skills/teams/scripts/monitor/service.py` L734-L741).

use std::sync::Arc;

use parking_lot::RwLock;
use uuid::Uuid;

use crate::error::{MaenifoldError, Result};
use crate::teams::trouter_protocol::{TrouterConnected, register};
use crate::teams::TrouterClientConfig;

#[derive(Clone)]
pub struct ReRegisterHandle {
    pub(crate) config: TrouterClientConfig,
    pub(crate) epid: Uuid,
    pub(crate) connected: Arc<RwLock<Option<TrouterConnected>>>,
}

impl ReRegisterHandle {
    pub async fn re_register(&self, token: &str) -> Result<()> {
        let Some(connected) = self.connected.read().clone() else {
            return Err(MaenifoldError::Message(
                "trouter re_register called before connect".into(),
            ));
        };
        register(token, &connected, &self.epid, &self.config).await
    }
}
