use crate::error::{JSONRPC_INVALID_PARAMS, MaenifoldError, Result};
use crate::mcp::stdio_pool::{StdioPool, start_session};
use crate::mcp::stdio_pool_health::lock_state;
use crate::mcp::stdio_pool_tools::{collect_gateway_tools, ensure_replacement_tools};
use serde_json::Value;
use std::sync::Arc;

impl StdioPool {
    pub(crate) async fn restart_server(&self, server_name: &str) -> Result<()> {
        let (config, restart_lock, restart_state) = {
            let servers = self.servers.read().await;
            let server = servers
                .get(server_name)
                .ok_or_else(|| missing_server(server_name))?;
            (
                server.config.clone(),
                server.restart_lock.clone(),
                server.restart_state.clone(),
            )
        };
        let _guard = restart_lock.lock().await;
        {
            let servers = self.servers.read().await;
            let server = servers
                .get(server_name)
                .ok_or_else(|| missing_server(server_name))?;
            if server
                .session
                .as_ref()
                .is_some_and(|session| session.is_usable())
            {
                return Ok(());
            }
        }
        if let Some(message) = lock_state(&restart_state).backoff_error(server_name) {
            return Err(MaenifoldError::Message(message));
        }
        let mut replacement = match start_session(&config).await {
            Ok(session) => Some(Arc::new(session)),
            Err(error) => {
                lock_state(&restart_state).record_failure(error.to_string());
                return Err(error);
            }
        };
        lock_state(&restart_state).record_success();
        let outcome = {
            let mut servers = self.servers.write().await;
            match servers.get(server_name) {
                None => Err(missing_server(server_name)),
                Some(server)
                    if server
                        .session
                        .as_ref()
                        .is_some_and(|session| session.is_usable()) =>
                {
                    Ok((None, None))
                }
                Some(_) => {
                    let Some(new_session) = replacement.as_ref() else {
                        return Err(MaenifoldError::Message(
                            "replacement stdio session missing".to_string(),
                        ));
                    };
                    ensure_replacement_tools(&servers, server_name, new_session.tools().as_ref())?;
                    let old = {
                        let server = servers
                            .get_mut(server_name)
                            .ok_or_else(|| missing_server(server_name))?;
                        std::mem::replace(
                            &mut server.session,
                            Some(replacement.take().ok_or_else(|| {
                                MaenifoldError::Message(
                                    "replacement stdio session missing".to_string(),
                                )
                            })?),
                        )
                    };
                    Ok((Some(old), Some(collect_gateway_tools(&servers)?)))
                }
            }
        };
        let (old_session, tool_state) = match outcome {
            Ok(outcome) => outcome,
            Err(error) => {
                if let Some(session) = replacement {
                    session.shutdown().await;
                }
                lock_state(&restart_state).record_failure(error.to_string());
                return Err(error);
            }
        };
        if let Some((tools, gateway_tools)) = tool_state {
            *self
                .tools
                .write()
                .unwrap_or_else(|error| error.into_inner()) = tools;
            *self
                .gateway_tools
                .write()
                .unwrap_or_else(|error| error.into_inner()) = gateway_tools;
        }
        if let Some(session) = replacement {
            session.shutdown().await;
        }
        if let Some(Some(session)) = old_session {
            session.shutdown().await;
        }
        Ok(())
    }
}

fn missing_server(server_name: &str) -> MaenifoldError {
    MaenifoldError::JsonRpc {
        code: JSONRPC_INVALID_PARAMS,
        message: format!("Gateway: unknown server '{server_name}'"),
        data: Value::Null,
    }
}
