use std::sync::Arc;

use crate::acp_config::{AgentsConfig, MaenifoldSettings};
use crate::acp_native::{AcpxResponse, AgentError, AgentHandle, AgentPool, SessionOpts};
use crate::error::{MaenifoldError, Result};
use agent_client_protocol as acp;
use tokio::runtime::{Builder, Handle};

pub(crate) struct AgentSession {
    handle: Arc<AgentHandle>,
    remote_id: String,
    cleanup_on_drop: bool,
}

pub(crate) async fn open_agent_session(
    pool: &AgentPool,
    agent_name: &str,
    _session_name: &str,
    mut opts: SessionOpts,
) -> Result<AgentSession> {
    let config = AgentsConfig::load(&opts.cwd);
    let config_entry = config.agents.get(agent_name).ok_or_else(|| {
        MaenifoldError::AgentSetup(format!(
            "agent '{}' not configured in maenifold.json",
            agent_name
        ))
    })?;
    let command = config_entry.command.clone();
    let env = config_entry.env.clone();
    if opts.mcp_servers.is_empty() {
        let mut servers: Vec<_> = config.mcp_servers.iter().map(|s| s.to_acp()).collect();
        servers.extend(config_entry.mcp_servers.iter().map(|s| s.to_acp()));
        opts.mcp_servers = servers;
    }
    if opts.acp_connection_timeout_seconds.is_none() {
        opts.acp_connection_timeout_seconds = Some(
            config
                .settings
                .as_ref()
                .map(|settings| settings.acp_connection_timeout_seconds)
                .unwrap_or_else(|| MaenifoldSettings::default().acp_connection_timeout_seconds),
        );
    }
    let handle = pool
        .get_or_create(
            agent_name,
            &command,
            &opts.cwd,
            env,
            opts.auto_grant_permissions,
        )
        .await;
    let remote_id = match opts.session_id.take() {
        Some(session_id) => {
            handle
                .load_session(&session_id, opts)
                .await
                .map_err(session_setup_error)?
                .session_id
        }
        None => {
            handle
                .new_session(opts)
                .await
                .map_err(session_setup_error)?
                .session_id
        }
    };
    Ok(AgentSession {
        handle,
        remote_id,
        cleanup_on_drop: true,
    })
}

impl AgentSession {
    pub(crate) fn remote_id(&self) -> &str {
        &self.remote_id
    }

    pub(crate) fn keep_alive(mut self) {
        self.cleanup_on_drop = false;
    }

    pub(crate) async fn prompt(&self, prompt: &str) -> Result<AcpxResponse> {
        self.prompt_with_context(prompt, vec![]).await
    }

    pub(crate) async fn prompt_with_context(
        &self,
        prompt: &str,
        context: Vec<acp::ContentBlock>,
    ) -> Result<AcpxResponse> {
        let response = self
            .handle
            .prompt_with_context(&self.remote_id, prompt, context)
            .await
            .map_err(prompt_execution_error)?;
        Ok(response)
    }

    pub(crate) async fn close(mut self) -> Result<()> {
        self.cleanup_on_drop = false;
        if let Err(error) = self.handle.close_session(&self.remote_id).await {
            tracing::warn!(
                session_id = %self.remote_id,
                error = %error,
                "Failed to close ACP session"
            );
        }
        Ok(())
    }
}

impl Drop for AgentSession {
    fn drop(&mut self) {
        if !self.cleanup_on_drop {
            return;
        }

        let handle = Arc::clone(&self.handle);
        let session_id = self.remote_id.clone();
        let cleanup = async move {
            handle.forget_session(&session_id).await;
            tracing::warn!(
                session_id = %session_id,
                "ACP session dropped without explicit close (for example after timeout); removed local session tracking"
            );
        };

        if let Ok(runtime) = Handle::try_current() {
            runtime.spawn(cleanup);
            return;
        }

        std::thread::spawn(move || {
            if let Ok(runtime) = Builder::new_current_thread().enable_all().build() {
                runtime.block_on(cleanup);
            }
        });
    }
}

fn session_setup_error(error: AgentError) -> MaenifoldError {
    MaenifoldError::AgentSetup(error.to_string())
}

fn prompt_execution_error(error: AgentError) -> MaenifoldError {
    MaenifoldError::Message(error.to_string())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::acp_native::test_handle_for_tests;

    use super::*;

    #[tokio::test]
    async fn keep_alive_preserves_tracked_session_without_queue_traffic() {
        let (handle, mut request_rx, mut control_rx) = test_handle_for_tests(1);
        handle.register_test_session("session-keep-alive").await;
        let handle = Arc::new(handle);
        let session = AgentSession {
            handle: Arc::clone(&handle),
            remote_id: "session-keep-alive".to_string(),
            cleanup_on_drop: true,
        };

        session.keep_alive();

        assert!(
            request_rx.try_recv().is_err(),
            "keep_alive should not enqueue request commands"
        );
        assert!(
            control_rx.try_recv().is_err(),
            "keep_alive should not enqueue control commands"
        );
        assert!(
            handle.has_tracked_session("session-keep-alive").await,
            "keep_alive should preserve tracked ACP session state"
        );
    }

    #[tokio::test]
    async fn drop_cleans_up_tracked_session_without_queue_traffic() {
        let (handle, mut request_rx, mut control_rx) = test_handle_for_tests(1);
        handle.register_test_session("session-drop-cleanup").await;
        let handle = Arc::new(handle);
        let session = AgentSession {
            handle: Arc::clone(&handle),
            remote_id: "session-drop-cleanup".to_string(),
            cleanup_on_drop: true,
        };

        drop(session);

        tokio::time::timeout(std::time::Duration::from_millis(50), async {
            loop {
                if !handle.has_tracked_session("session-drop-cleanup").await {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("drop cleanup should remove tracked session");

        assert!(
            request_rx.try_recv().is_err(),
            "drop cleanup should not enqueue request commands"
        );
        assert!(
            control_rx.try_recv().is_err(),
            "drop cleanup should not enqueue control commands"
        );
    }
}
