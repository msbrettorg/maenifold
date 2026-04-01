#![allow(dead_code)]

use chrono::{Duration, Utc};
use maenifold_core::{Config, memory::compute_decay_weight};
use tempfile::TempDir;

// T-12.7: RTM NFR-1
pub(crate) use crate::GLOBAL_CONFIG_TEST_MUTEX as TEST_MUTEX;

pub struct TestEnv {
    _tmp: TempDir,
    pub cfg: Config,
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
    pub fn with_values(
        decay_function: &str,
        half_life_days: i64,
        sequential_grace_days: i64,
        workflows_grace_days: i64,
        default_grace_days: i64,
    ) -> Self {
        let keys = [
            "MAENIFOLD_DECAY_FUNCTION",
            "MAENIFOLD_DECAY_HALF_LIFE_DAYS",
            "MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL",
            "MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS",
            "MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT",
        ];

        let previous = keys
            .into_iter()
            .map(|key| (key, std::env::var(key).ok()))
            .collect::<Vec<_>>();

        // SAFETY: all tests in this split suite hold TEST_MUTEX for their full duration,
        // so these process-wide environment mutations are serialized.
        unsafe {
            std::env::set_var("MAENIFOLD_DECAY_FUNCTION", decay_function);
            std::env::set_var("MAENIFOLD_DECAY_HALF_LIFE_DAYS", half_life_days.to_string());
            std::env::set_var(
                "MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL",
                sequential_grace_days.to_string(),
            );
            std::env::set_var(
                "MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS",
                workflows_grace_days.to_string(),
            );
            std::env::set_var(
                "MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT",
                default_grace_days.to_string(),
            );
        }

        Self { previous }
    }

    pub fn exponential_defaults() -> Self {
        Self::with_values("exponential", 30, 7, 14, 28)
    }

    pub fn power_law_defaults() -> Self {
        Self::with_values("power-law", 30, 7, 14, 28)
    }
}

impl Drop for DecayEnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.previous {
            if let Some(value) = value {
                // SAFETY: all tests in this split suite hold TEST_MUTEX for their full duration,
                // so these process-wide environment mutations are serialized.
                unsafe {
                    std::env::set_var(key, value);
                }
            } else {
                // SAFETY: all tests in this split suite hold TEST_MUTEX for their full duration,
                // so these process-wide environment mutations are serialized.
                unsafe {
                    std::env::remove_var(key);
                }
            }
        }
    }
}

pub fn isolated_env() -> TestEnv {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    TestEnv { _tmp: tmp, cfg }
}

pub fn weight_for(
    cfg: &Config,
    file_path: &str,
    days_ago: Option<i64>,
    status: Option<&str>,
) -> f64 {
    let last_accessed = days_ago.map(rfc3339_days_ago);
    compute_decay_weight(file_path, last_accessed.as_deref(), cfg, status)
}

pub fn assert_approx_eq(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected} ± {tolerance}, got {actual}"
    );
}

fn rfc3339_days_ago(days_ago: i64) -> String {
    (Utc::now() - Duration::days(days_ago)).to_rfc3339()
}
