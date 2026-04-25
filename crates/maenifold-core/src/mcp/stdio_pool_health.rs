use crate::mcp::gateway_schema_validation::compute_schema_hash;
use crate::mcp::stdio_pool::StdioPool;
use serde_json::Value;
use std::sync::{Mutex as StdMutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime};

const BACKOFF_BASE: Duration = Duration::from_secs(1);
const BACKOFF_MAX: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StdioPoolHealth {
    pub servers: Vec<StdioServerHealth>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StdioServerHealth {
    pub name: String,
    pub status: ServerStartupStatus,
    pub configured: bool,
    pub enabled: bool,
    pub running: bool,
    pub degraded: bool,
    pub last_start_error: Option<String>,
    pub stderr_tail: Vec<String>,
    pub last_restart: Option<SystemTime>,
    pub tool_count: usize,
    pub tool_schema_hashes: Vec<(String, String)>,
    pub env_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerStartupStatus {
    Started,
    Failed,
}

#[derive(Debug, Clone)]
pub(crate) struct RestartState {
    pub(crate) last_error: Option<String>,
    pub(crate) last_stderr: Option<String>,
    pub(crate) last_restart: Option<SystemTime>,
    pub(crate) consecutive_failures: u32,
    pub(crate) backoff_until: Option<Instant>,
}

impl RestartState {
    pub(crate) fn new() -> Self {
        Self {
            last_error: None,
            last_stderr: None,
            last_restart: None,
            consecutive_failures: 0,
            backoff_until: None,
        }
    }

    pub(crate) fn record_success(&mut self) {
        self.last_error = None;
        self.last_stderr = None;
        self.last_restart = Some(SystemTime::now());
        self.consecutive_failures = 0;
        self.backoff_until = None;
    }

    pub(crate) fn record_failure(&mut self, error: String, stderr: Option<String>) {
        self.last_error = Some(error);
        self.last_stderr = stderr;
        self.last_restart = Some(SystemTime::now());
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        self.backoff_until = Some(Instant::now() + failure_backoff(self.consecutive_failures));
    }

    pub(crate) fn backoff_error(&self, server_name: &str) -> Option<String> {
        let until = self.backoff_until?;
        let remaining = until.checked_duration_since(Instant::now())?;
        let last_error = self
            .last_error
            .as_deref()
            .unwrap_or("unknown start failure");
        Some(format!(
            "Gateway: server '{server_name}' restart backoff active for {}s after previous start failure: {last_error}",
            remaining.as_secs().max(1)
        ))
    }
}

impl StdioPool {
    pub async fn health(&self) -> StdioPoolHealth {
        let servers = self.servers.read().await;
        let mut health = servers
            .values()
            .map(|server| {
                let state = lock_state(&server.restart_state).clone();
                let running = server
                    .session
                    .as_ref()
                    .is_some_and(|session| session.is_usable());
                StdioServerHealth {
                    name: server.config.name.clone(),
                    status: if running {
                        ServerStartupStatus::Started
                    } else {
                        ServerStartupStatus::Failed
                    },
                    configured: true,
                    enabled: true,
                    running,
                    degraded: !running || state.last_error.is_some(),
                    last_start_error: state.last_error.map(|error| sanitize_error(&error)),
                    stderr_tail: stderr_tail(state.last_stderr.as_deref()),
                    last_restart: state.last_restart,
                    tool_count: server
                        .session
                        .as_ref()
                        .map_or(0, |session| session.tools().len()),
                    tool_schema_hashes: server.session.as_ref().map_or_else(Vec::new, |session| {
                        let mut hashes = session
                            .tools()
                            .iter()
                            .map(|tool| {
                                let schema = Value::Object((*tool.input_schema).clone());
                                (tool.name.to_string(), compute_schema_hash(&schema))
                            })
                            .collect::<Vec<_>>();
                        hashes.sort_by(|left, right| left.0.cmp(&right.0));
                        hashes
                    }),
                    env_keys: redacted_env_keys(&server.config.env),
                }
            })
            .collect::<Vec<_>>();
        health.sort_by(|left, right| left.name.cmp(&right.name));
        StdioPoolHealth { servers: health }
    }
}

pub(crate) fn lock_state(state: &StdMutex<RestartState>) -> MutexGuard<'_, RestartState> {
    state.lock().unwrap_or_else(|error| error.into_inner())
}

fn redacted_env_keys(env: &std::collections::HashMap<String, String>) -> Vec<String> {
    let mut keys = env.keys().cloned().collect::<Vec<_>>();
    keys.sort();
    keys
}

pub(crate) fn sanitize_error(msg: &str) -> String {
    let chars = msg.char_indices().collect::<Vec<_>>();
    let mut sanitized = String::with_capacity(msg.len());
    let mut last = 0;
    let mut index = 0;
    while index < chars.len() {
        let (byte_index, ch) = chars[index];
        let is_home_path = ch == '~' && chars.get(index + 1).is_some_and(|(_, next)| *next == '/');
        let is_absolute_path = ch == '/';
        if (is_home_path || is_absolute_path) && is_path_boundary(msg, byte_index) {
            sanitized.push_str(&msg[last..byte_index]);
            sanitized.push_str("[path]");
            index += 1;
            while index < chars.len() && !is_path_terminator(chars[index].1) {
                index += 1;
            }
            last = chars.get(index).map_or(msg.len(), |(next, _)| *next);
        } else {
            index += 1;
        }
    }
    sanitized.push_str(&msg[last..]);
    sanitized
}

fn is_path_boundary(msg: &str, byte_index: usize) -> bool {
    msg[..byte_index].chars().next_back().is_none_or(|ch| {
        ch.is_whitespace() || matches!(ch, '\'' | '"' | '(' | '[' | '{' | '=' | ':' | '`')
    })
}

fn is_path_terminator(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, '\'' | '"' | ')' | ']' | '}' | ',' | ';')
}

fn stderr_tail(stderr: Option<&str>) -> Vec<String> {
    let Some(stderr) = stderr else {
        return Vec::new();
    };
    let mut tail = stderr
        .lines()
        .rev()
        .take(10)
        .map(sanitize_error)
        .collect::<Vec<_>>();
    tail.reverse();
    tail
}

fn failure_backoff(consecutive_failures: u32) -> Duration {
    let exponent = consecutive_failures.saturating_sub(1).min(5);
    BACKOFF_BASE
        .checked_mul(1_u32 << exponent)
        .unwrap_or(BACKOFF_MAX)
        .min(BACKOFF_MAX)
}
