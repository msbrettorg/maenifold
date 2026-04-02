/// Agent pool and high-level ACP driver.
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, LazyLock},
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

use super::{AgentCommand, AgentHandle, spawn_agent};

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
        auto_grant_permissions: bool,
    ) -> Arc<AgentHandle> {
        let pool_key = format!("{agent_name}::auto-grant={auto_grant_permissions}");
        let mut agents = self.agents.lock().await;
        if let Some(entry) = agents.get_mut(&pool_key) {
            if entry.handle.is_alive() {
                entry.last_used = Instant::now();
                return Arc::clone(&entry.handle);
            }
        }
        agents.remove(&pool_key);

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

        let handle = Arc::new(spawn_agent(
            command.to_string(),
            cwd.to_path_buf(),
            env,
            auto_grant_permissions,
        ));
        agents.insert(
            pool_key,
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
