/// Agent pool and high-level ACP driver.
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::Mutex;

use crate::acp_config::AgentsConfig;

use super::{
    AcpxResponse, AgentCommand, AgentError, AgentHandle, Result, SessionOpts, spawn_agent,
};

pub struct AgentPool {
    agents: Mutex<HashMap<String, Arc<AgentHandle>>>,
}

impl AgentPool {
    pub fn new() -> Self {
        Self {
            agents: Mutex::new(HashMap::new()),
        }
    }
    pub async fn get_or_create(
        &self,
        agent_name: &str,
        command: &str,
        cwd: &Path,
        env: HashMap<String, String>,
    ) -> Arc<AgentHandle> {
        let mut agents = self.agents.lock().await;
        if let Some(handle) = agents.get(agent_name) {
            if handle.is_alive() {
                return Arc::clone(handle);
            }
        }
        agents.remove(agent_name);
        let handle = Arc::new(spawn_agent(command.to_string(), cwd.to_path_buf(), env));
        agents.insert(agent_name.to_string(), Arc::clone(&handle));
        handle
    }
    pub async fn close_all(&self) {
        for (_, handle) in self.agents.lock().await.drain() {
            let _ = handle.cmd_tx.send(AgentCommand::Shutdown);
        }
    }
}

impl Default for AgentPool {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AcpxDriver {
    command: String,
    env: HashMap<String, String>,
    cwd: PathBuf,
    timeout_seconds: u64,
    pool: AgentPool,
    sessions: Mutex<HashMap<String, SessionRef>>,
}

impl std::fmt::Debug for AcpxDriver {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AcpxDriver")
            .field("command", &self.command)
            .field("env", &self.env)
            .field("cwd", &self.cwd)
            .field("timeout_seconds", &self.timeout_seconds)
            .finish_non_exhaustive()
    }
}

struct SessionRef {
    handle: Arc<AgentHandle>,
    remote_id: String,
}

impl AcpxDriver {
    fn map_cached_session_error(
        handle: &AgentHandle,
        session_id: &str,
        error: AgentError,
    ) -> AgentError {
        match error {
            AgentError::ChannelClosed if !handle.is_alive() => AgentError::AgentDied {
                session_id: session_id.to_string(),
            },
            other => other,
        }
    }

    pub fn new(agent: String, timeout_seconds: u64, cwd: PathBuf) -> Result<Self> {
        let config_entry = AgentsConfig::load(&cwd)
            .agents
            .get(&agent)
            .cloned()
            .ok_or_else(|| {
                AgentError::NotFound(format!("agent '{}' not configured in agents.json", agent))
            })?;
        Ok(Self {
            command: config_entry.command,
            env: config_entry.env,
            cwd,
            timeout_seconds,
            pool: AgentPool::new(),
            sessions: Mutex::new(HashMap::new()),
        })
    }
    pub async fn create_session(&self, name: &str) -> Result<()> {
        let handle = self
            .pool
            .get_or_create(&self.command, &self.command, &self.cwd, self.env.clone())
            .await;
        let remote_id = handle
            .new_session(SessionOpts {
                cwd: self.cwd.clone(),
                model: None,
                mode: None,
                thought_level: None,
                config_options: None,
                allowed_tools: None,
                max_turns: None,
                timeout_seconds: Some(self.timeout_seconds),
                mcp_servers: vec![],
            })
            .await?
            .session_id;
        let previous = self.sessions.lock().await.insert(
            name.to_string(),
            SessionRef {
                handle: Arc::clone(&handle),
                remote_id,
            },
        );
        if let Some(previous) = previous {
            previous.handle.close_session(&previous.remote_id).await;
        }
        Ok(())
    }
    pub async fn prompt(&self, session: &str, prompt: &str) -> Result<AcpxResponse> {
        let sessions = self.sessions.lock().await;
        let current = sessions
            .get(session)
            .ok_or_else(|| AgentError::Acp(format!("Unknown session '{session}'")))?;
        let (handle, remote_id) = (Arc::clone(&current.handle), current.remote_id.clone());
        drop(sessions);
        let response = handle
            .prompt(&remote_id, prompt)
            .await
            .map_err(|error| Self::map_cached_session_error(&handle, session, error))?;
        Ok(AcpxResponse {
            text: response.text,
            tool_calls: response.tool_calls,
            thought: response.thought,
            error: None,
            usage: response.usage,
        })
    }
    pub async fn close_session(&self, name: &str) -> Result<()> {
        if let Some(session) = self.sessions.lock().await.remove(name) {
            session.handle.close_session(&session.remote_id).await;
        }
        Ok(())
    }
    pub async fn cancel(&self, session: &str) -> Result<()> {
        let sessions = self.sessions.lock().await;
        let current = sessions
            .get(session)
            .ok_or_else(|| AgentError::Acp(format!("Unknown session '{session}'")))?;
        let (handle, remote_id) = (Arc::clone(&current.handle), current.remote_id.clone());
        drop(sessions);
        handle
            .cancel(&remote_id)
            .await
            .map_err(|error| Self::map_cached_session_error(&handle, session, error))
    }
}

#[cfg(test)]
mod tests {
    use super::{AcpxDriver, AgentError};
    use tempfile::tempdir;

    #[test]
    fn new_returns_not_found_when_agent_missing_from_config() {
        let tmp = tempdir().expect("tempdir");
        let error = AcpxDriver::new("missing-agent".to_string(), 0, tmp.path().to_path_buf())
            .expect_err("missing agent should error");
        assert!(
            matches!(error, AgentError::NotFound(message) if message.contains("missing-agent") && message.contains("agents.json"))
        );
    }
}
