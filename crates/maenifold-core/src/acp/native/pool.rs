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
            max_agents: 20,
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
            let candidates = agents
                .iter()
                .map(|(name, entry)| (name.clone(), Arc::clone(&entry.handle), entry.last_used))
                .collect::<Vec<_>>();

            let mut evicted_name = candidates
                .iter()
                .find(|(_, handle, _)| !handle.is_alive())
                .map(|(name, _, _)| name.clone());

            if evicted_name.is_none() {
                let mut oldest: Option<(PoolKey, Instant)> = None;
                for (name, handle, last_used) in candidates {
                    if handle.has_tracked_sessions().await {
                        continue;
                    }
                    match oldest {
                        Some((_, oldest_last_used)) if oldest_last_used <= last_used => {}
                        _ => oldest = Some((name, last_used)),
                    }
                }
                evicted_name = oldest.map(|(name, _)| name);
            }

            if let Some(name) = evicted_name {
                let should_evict = if let Some(entry) = agents.get(&name) {
                    let handle = Arc::clone(&entry.handle);
                    !handle.is_alive() || !handle.has_tracked_sessions().await
                } else {
                    false
                };

                if should_evict && let Some(entry) = agents.remove(&name) {
                    let _ = entry.handle.shutdown_nowait();
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

    pub async fn shutdown_handle(&self, handle: &Arc<AgentHandle>) -> bool {
        let mut agents = self.agents.lock().await;
        let key = agents
            .iter()
            .find_map(|(key, entry)| Arc::ptr_eq(&entry.handle, handle).then_some(key.clone()));
        let Some(key) = key else {
            return false;
        };
        if let Some(entry) = agents.remove(&key) {
            let _ = entry.handle.shutdown_nowait();
            true
        } else {
            false
        }
    }

    pub async fn touch_handle(&self, handle: &Arc<AgentHandle>) -> bool {
        let mut agents = self.agents.lock().await;
        let Some((_, entry)) = agents
            .iter_mut()
            .find(|(_, entry)| Arc::ptr_eq(&entry.handle, handle))
        else {
            return false;
        };
        entry.last_used = Instant::now();
        true
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
    use crate::acp_native::test_handle_for_tests;

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

    #[tokio::test]
    async fn eviction_skips_handles_with_tracked_sessions() {
        let pool = AgentPool {
            agents: Mutex::new(HashMap::new()),
            max_agents: 1,
        };
        let (tracked_handle, _request_rx, _control_rx) = test_handle_for_tests(1);
        let tracked_handle = Arc::new(tracked_handle);
        tracked_handle.register_test_session("session-in-use").await;

        let tracked_key = AgentPool::pool_key(
            "tracked",
            "tracked --serve",
            Path::new("."),
            &HashMap::new(),
            false,
        );
        pool.agents.lock().await.insert(
            tracked_key,
            PoolEntry {
                handle: Arc::clone(&tracked_handle),
                last_used: Instant::now() - Duration::from_secs(60),
            },
        );

        let returned = pool
            .get_or_create(
                "fresh",
                "fresh --serve",
                Path::new("."),
                HashMap::new(),
                false,
            )
            .await;

        assert!(
            !Arc::ptr_eq(&returned, &tracked_handle),
            "pool should not evict a handle with live tracked sessions"
        );
        assert!(
            tracked_handle.has_tracked_sessions().await,
            "tracked handle should remain live in the pool"
        );
        assert_eq!(
            pool.agents.lock().await.len(),
            2,
            "pool should retain the tracked handle and admit the new one"
        );
        pool.close_all().await;
    }

    #[tokio::test]
    async fn touch_handle_refreshes_last_used() {
        let pool = AgentPool {
            agents: Mutex::new(HashMap::new()),
            max_agents: 1,
        };
        let (handle, _request_rx, _control_rx) = test_handle_for_tests(1);
        let handle = Arc::new(handle);
        let key = AgentPool::pool_key(
            "tracked",
            "tracked --serve",
            Path::new("."),
            &HashMap::new(),
            false,
        );
        let stale = Instant::now() - Duration::from_secs(60);
        pool.agents.lock().await.insert(
            key.clone(),
            PoolEntry {
                handle: Arc::clone(&handle),
                last_used: stale,
            },
        );

        assert!(pool.touch_handle(&handle).await);

        let refreshed = pool
            .agents
            .lock()
            .await
            .get(&key)
            .expect("entry must still exist")
            .last_used;
        assert!(
            refreshed > stale,
            "touch_handle should update last_used for active relay traffic"
        );
        pool.close_all().await;
    }
}
