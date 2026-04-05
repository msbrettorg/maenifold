/// Agent pool and high-level ACP driver.
use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
    sync::{Arc, LazyLock},
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

use super::{AgentHandle, spawn_agent};

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PoolKey {
    agent_name: String,
    command: String,
    cwd: PathBuf,
    env: Vec<(String, String)>,
    auto_grant_permissions: bool,
}

pub struct AgentPool {
    agents: Mutex<HashMap<PoolKey, PoolEntry>>,
    max_agents: usize,
}

impl AgentPool {
    pub fn new() -> Self {
        Self {
            agents: Mutex::new(HashMap::new()),
            max_agents: 10,
        }
    }

    fn pool_key(
        agent_name: &str,
        command: &str,
        cwd: &Path,
        env: &HashMap<String, String>,
        auto_grant_permissions: bool,
    ) -> PoolKey {
        let mut env_pairs: Vec<(String, String)> = env
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        env_pairs.sort_unstable();

        PoolKey {
            agent_name: agent_name.to_string(),
            command: command.to_string(),
            cwd: normalize_pool_cwd(cwd),
            env: env_pairs,
            auto_grant_permissions,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn get_or_create(
        &self,
        agent_name: &str,
        command: &str,
        cwd: &Path,
        env: HashMap<String, String>,
        auto_grant_permissions: bool,
    ) -> Arc<AgentHandle> {
        let pool_key = Self::pool_key(agent_name, command, cwd, &env, auto_grant_permissions);
        let mut agents = self.agents.lock().await;
        if let Some(entry) = agents.get_mut(&pool_key)
            && entry.handle.is_alive()
        {
            entry.last_used = Instant::now();
            return Arc::clone(&entry.handle);
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

            if let Some(name) = evicted_name
                && let Some(entry) = agents.remove(&name)
            {
                let _ = entry.handle.shutdown_nowait();
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
        self.agents
            .lock()
            .await
            .retain(|_, entry| entry.handle.is_alive());
    }
    pub async fn close_all(&self) {
        for (_, entry) in self.agents.lock().await.drain() {
            let _ = entry.handle.shutdown_nowait();
        }
    }
}

impl Default for AgentPool {
    fn default() -> Self {
        Self::new()
    }
}

fn normalize_pool_cwd(cwd: &Path) -> PathBuf {
    let normalized = lexical_normalize_path(cwd);
    canonicalize_existing_or_parent(&normalized).unwrap_or(normalized)
}

fn canonicalize_existing_or_parent(path: &Path) -> std::io::Result<PathBuf> {
    if let Ok(canonical) = std::fs::canonicalize(path) {
        return Ok(canonical);
    }

    let mut tail = Vec::new();
    let mut current = path.to_path_buf();

    loop {
        if let Some(name) = current.file_name() {
            tail.push(name.to_os_string());
        }

        match current.parent() {
            Some(parent) if parent != current => current = parent.to_path_buf(),
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("no existing ancestor for path: {}", path.display()),
                ));
            }
        }

        match std::fs::canonicalize(&current) {
            Ok(canonical_ancestor) => {
                let mut result = canonical_ancestor;
                for component in tail.into_iter().rev() {
                    result.push(component);
                }
                return Ok(lexical_normalize_path(&result));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        }
    }
}

fn lexical_normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    let mut is_absolute = false;

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => {
                normalized.push(component.as_os_str());
                is_absolute = true;
            }
            Component::CurDir => {}
            Component::ParentDir => {
                let can_pop = normalized.file_name().is_some();
                if can_pop {
                    normalized.pop();
                } else if !is_absolute {
                    normalized.push(component.as_os_str());
                }
            }
            Component::Normal(part) => normalized.push(part),
        }
    }

    if normalized.as_os_str().is_empty() {
        if is_absolute {
            PathBuf::from(std::path::MAIN_SEPARATOR.to_string())
        } else {
            PathBuf::from(".")
        }
    } else {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_pairs(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        let mut env = HashMap::new();
        for (key, value) in pairs {
            env.insert((*key).to_string(), (*value).to_string());
        }
        env
    }

    #[test]
    fn pool_key_changes_when_command_changes() {
        let cwd = std::env::current_dir().expect("read current dir");
        let env = env_pairs(&[("ALPHA", "1"), ("BETA", "2")]);

        let first = AgentPool::pool_key("agent", "alpha --serve", &cwd, &env, true);
        let second = AgentPool::pool_key("agent", "beta --serve", &cwd, &env, true);

        assert_ne!(first, second, "command must participate in pool identity");
    }

    #[test]
    fn pool_key_treats_equivalent_existing_cwds_the_same() {
        let cwd = std::env::current_dir().expect("read current dir");
        let alias = cwd.join(".");
        let env = env_pairs(&[("ALPHA", "1"), ("BETA", "2")]);

        let first = AgentPool::pool_key("agent", "alpha --serve", &cwd, &env, true);
        let second = AgentPool::pool_key("agent", "alpha --serve", &alias, &env, true);

        assert_eq!(
            first, second,
            "equivalent cwd paths should normalize to the same pool identity"
        );
    }

    #[test]
    fn pool_key_changes_when_cwd_changes() {
        let cwd = std::env::current_dir().expect("read current dir");
        let other_cwd = cwd.join("crates");
        let env = env_pairs(&[("ALPHA", "1"), ("BETA", "2")]);

        let first = AgentPool::pool_key("agent", "alpha --serve", &cwd, &env, true);
        let second = AgentPool::pool_key("agent", "alpha --serve", &other_cwd, &env, true);

        assert_ne!(first, second, "cwd must participate in pool identity");
    }

    #[test]
    fn pool_key_is_stable_across_env_insertion_order() {
        let cwd = std::env::current_dir().expect("read current dir");
        let first_env = env_pairs(&[("BETA", "2"), ("ALPHA", "1")]);
        let second_env = env_pairs(&[("ALPHA", "1"), ("BETA", "2")]);

        let first = AgentPool::pool_key("agent", "alpha --serve", &cwd, &first_env, true);
        let second = AgentPool::pool_key("agent", "alpha --serve", &cwd, &second_env, true);

        assert_eq!(
            first, second,
            "logical env identity must be stable across HashMap insertion order"
        );
    }

    #[test]
    fn pool_key_changes_when_env_changes() {
        let cwd = std::env::current_dir().expect("read current dir");
        let first_env = env_pairs(&[("ALPHA", "1"), ("BETA", "2")]);
        let second_env = env_pairs(&[("ALPHA", "1"), ("BETA", "3")]);

        let first = AgentPool::pool_key("agent", "alpha --serve", &cwd, &first_env, true);
        let second = AgentPool::pool_key("agent", "alpha --serve", &cwd, &second_env, true);

        assert_ne!(
            first, second,
            "environment changes must produce a distinct pool identity"
        );
    }

    #[test]
    fn pool_key_changes_when_auto_grant_changes() {
        let cwd = std::env::current_dir().expect("read current dir");
        let env = env_pairs(&[("ALPHA", "1"), ("BETA", "2")]);

        let first = AgentPool::pool_key("agent", "alpha --serve", &cwd, &env, true);
        let second = AgentPool::pool_key("agent", "alpha --serve", &cwd, &env, false);

        assert_ne!(
            first, second,
            "auto-grant remains part of the existing pool identity contract"
        );
    }

    #[test]
    fn pool_key_normalizes_nonexistent_cwd_via_safe_fallback() {
        let cwd = std::env::current_dir()
            .expect("read current dir")
            .join("pool-key-missing-dir");
        let alias = cwd.join(".").join("child").join("..");
        let env = env_pairs(&[("ALPHA", "1"), ("BETA", "2")]);

        let first = AgentPool::pool_key("agent", "alpha --serve", &cwd, &env, true);
        let second = AgentPool::pool_key("agent", "alpha --serve", &alias, &env, true);

        assert_eq!(
            first, second,
            "nonexistent cwd aliases should normalize deterministically via safe fallback"
        );
    }
}
