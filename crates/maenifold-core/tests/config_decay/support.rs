#![allow(dead_code)]

// T-12.5: RTM NFR-1
use chrono::{Duration, Utc};
use maenifold_core::Config;
use maenifold_core::json::OutputFormat;
use maenifold_core::tools::{self, ToolContext};
use parking_lot::MutexGuard;
use serde_json::json;
use tempfile::TempDir;

pub(crate) use crate::GLOBAL_CONFIG_TEST_MUTEX as TEST_MUTEX;

const DECAY_ENV_KEYS: [&str; 5] = [
    "MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL",
    "MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS",
    "MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT",
    "MAENIFOLD_DECAY_HALF_LIFE_DAYS",
    "MAENIFOLD_DECAY_FUNCTION",
];

pub struct TestEnv {
    _tmp: TempDir,
    pub cfg: Config,
}

impl TestEnv {
    pub fn markdown_context(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Markdown)
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

pub struct DecayEnvGuard {
    previous: Vec<(&'static str, Option<String>)>,
}

impl DecayEnvGuard {
    fn capture() -> Self {
        let previous = DECAY_ENV_KEYS
            .into_iter()
            .map(|key| (key, std::env::var(key).ok()))
            .collect();
        Self { previous }
    }
}

impl Drop for DecayEnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.previous {
            match value {
                Some(value) => {
                    // SAFETY: all tests in this module hold TEST_MUTEX for their full duration,
                    // so process-wide environment mutations are serialized.
                    unsafe { std::env::set_var(key, value) };
                }
                None => {
                    // SAFETY: all tests in this module hold TEST_MUTEX for their full duration,
                    // so process-wide environment mutations are serialized.
                    unsafe { std::env::remove_var(key) };
                }
            }
        }
    }
}

pub fn acquire_test_lock() -> MutexGuard<'static, ()> {
    TEST_MUTEX.lock()
}

pub fn isolated_env() -> TestEnv {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    TestEnv { _tmp: tmp, cfg }
}

pub fn apply_decay_env(vars: &[(&'static str, Option<&str>)]) -> DecayEnvGuard {
    let guard = DecayEnvGuard::capture();
    for (key, value) in vars {
        match value {
            Some(value) => {
                // SAFETY: all tests in this module hold TEST_MUTEX for their full duration,
                // so process-wide environment mutations are serialized.
                unsafe { std::env::set_var(key, value) };
            }
            None => {
                // SAFETY: all tests in this module hold TEST_MUTEX for their full duration,
                // so process-wide environment mutations are serialized.
                unsafe { std::env::remove_var(key) };
            }
        }
    }
    guard
}

pub fn clear_decay_env() -> DecayEnvGuard {
    let vars = DECAY_ENV_KEYS.map(|key| (key, None));
    apply_decay_env(&vars)
}

pub fn config_summary(env: &TestEnv) -> String {
    tools::get_config(&env.markdown_context(), &json!({})).expect("get config")
}

pub fn rfc3339_days_ago(days_ago: i64) -> String {
    (Utc::now() - Duration::days(days_ago)).to_rfc3339()
}

pub fn assert_approx_eq(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected} ± {tolerance}, got {actual}"
    );
}
