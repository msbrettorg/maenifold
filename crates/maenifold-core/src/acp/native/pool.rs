/// Agent pool and high-level ACP driver.
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock},
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

use crate::acp_config::AgentsConfig;

use super::{
    AcpxResponse, AgentCommand, AgentError, AgentHandle, Result, SessionOpts, spawn_agent,
};

/// Global agent pool shared across all ACP code paths (MCP tools, pipeline,
/// CLI).  Agents are spawned on first use and reused until they die.
pub static GLOBAL_AGENT_POOL: LazyLock<Arc<AgentPool>> = LazyLock::new(|| {
    let pool = Arc::new(AgentPool::new());
    let sweeper_pool = Arc::clone(&pool);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            sweeper_pool.remove_dead().await;
        }
    });
    pool
});

struct PoolEntry {
    handle: Arc<AgentHandle>,
    last_used: Instant,
}

pub struct AgentPool {
    agents: Mutex<HashMap<String, PoolEntry>>,
    max_agents: usize,
}

impl AgentPool {
    pub fn new() -> Self {
        Self {
            agents: Mutex::new(HashMap::new()),
            max_agents: 10,
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
        if let Some(entry) = agents.get_mut(agent_name) {
            if entry.handle.is_alive() {
                entry.last_used = Instant::now();
                return Arc::clone(&entry.handle);
            }
        }
        agents.remove(agent_name);

        if agents.len() >= self.max_agents {
            let evicted_name = agents
                .iter()
                .find(|(_, entry)| !entry.handle.is_alive())
                .map(|(name, _)| name.clone())
                .or_else(|| {
                    agents
                        .iter()
                        .min_by_key(|(_, entry)| entry.last_used)
                        .map(|(name, _)| name.clone())
                });

            if let Some(name) = evicted_name {
                if let Some(entry) = agents.remove(&name) {
                    let _ = entry.handle.cmd_tx.send(AgentCommand::Shutdown);
                }
            }
        }

        let handle = Arc::new(spawn_agent(command.to_string(), cwd.to_path_buf(), env));
        agents.insert(
            agent_name.to_string(),
            PoolEntry {
                handle: Arc::clone(&handle),
                last_used: Instant::now(),
            },
        );
        handle
    }
    pub async fn remove_dead(&self) {
        self.agents.lock().await.retain(|_, entry| entry.handle.is_alive());
    }
    pub async fn close_all(&self) {
        for (_, entry) in self.agents.lock().await.drain() {
            let _ = entry.handle.cmd_tx.send(AgentCommand::Shutdown);
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
                timeout_seconds: None,
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
