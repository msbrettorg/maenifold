use chrono::{Duration, Utc};
use maenifold_core::Config;
use maenifold_core::json::OutputFormat;
use maenifold_core::memory;
use maenifold_core::tools::{self, ToolContext};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::json;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

const DECAY_ENV_KEYS: [&str; 5] = [
    "MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL",
    "MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS",
    "MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT",
    "MAENIFOLD_DECAY_HALF_LIFE_DAYS",
    "MAENIFOLD_DECAY_FUNCTION",
];

struct TestEnv {
    _tmp: TempDir,
    cfg: Config,
}

impl TestEnv {
    fn markdown_context(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Markdown)
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

struct DecayEnvGuard {
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
                    // SAFETY: all tests in this file hold TEST_MUTEX for their full duration,
                    // so process-wide environment mutations are serialized.
                    unsafe { std::env::set_var(key, value) };
                }
                None => {
                    // SAFETY: all tests in this file hold TEST_MUTEX for their full duration,
                    // so process-wide environment mutations are serialized.
                    unsafe { std::env::remove_var(key) };
                }
            }
        }
    }
}

fn isolated_env() -> TestEnv {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    TestEnv { _tmp: tmp, cfg }
}

fn apply_decay_env(vars: &[(&'static str, Option<&str>)]) -> DecayEnvGuard {
    let guard = DecayEnvGuard::capture();
    for (key, value) in vars {
        match value {
            Some(value) => {
                // SAFETY: all tests in this file hold TEST_MUTEX for their full duration,
                // so process-wide environment mutations are serialized.
                unsafe { std::env::set_var(key, value) };
            }
            None => {
                // SAFETY: all tests in this file hold TEST_MUTEX for their full duration,
                // so process-wide environment mutations are serialized.
                unsafe { std::env::remove_var(key) };
            }
        }
    }
    guard
}

fn clear_decay_env() -> DecayEnvGuard {
    let vars = DECAY_ENV_KEYS.map(|key| (key, None));
    apply_decay_env(&vars)
}

fn config_summary(env: &TestEnv) -> String {
    tools::get_config(&env.markdown_context(), &json!({})).expect("get config")
}

fn rfc3339_days_ago(days_ago: i64) -> String {
    (Utc::now() - Duration::days(days_ago)).to_rfc3339()
}

fn assert_approx_eq(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected} ± {tolerance}, got {actual}"
    );
}

// T-GRAPH-DECAY-001.2: RTM NFR-7.5.1
#[test]
fn decay_grace_days_sequential_default_is_seven() {
    let _guard = TEST_MUTEX.lock();
    let _env = clear_decay_env();
    let env = isolated_env();

    assert_eq!(env.cfg.decay_grace_days_sequential(), 7);
}

// T-GRAPH-DECAY-001.2a: RTM NFR-7.5.1a
#[test]
fn decay_grace_days_workflows_default_is_fourteen() {
    let _guard = TEST_MUTEX.lock();
    let _env = clear_decay_env();
    let env = isolated_env();

    assert_eq!(env.cfg.decay_grace_days_workflows(), 14);
}

// T-GRAPH-DECAY-001.3: RTM NFR-7.5.2
#[test]
fn decay_grace_days_default_default_is_twenty_eight() {
    let _guard = TEST_MUTEX.lock();
    let _env = clear_decay_env();
    let env = isolated_env();

    assert_eq!(env.cfg.decay_grace_days_default(), 28);
}

// T-GRAPH-DECAY-001.4: RTM NFR-7.5.3
#[test]
fn decay_half_life_days_default_is_thirty() {
    let _guard = TEST_MUTEX.lock();
    let _env = clear_decay_env();
    let env = isolated_env();

    assert_eq!(env.cfg.decay_half_life_days(), 30);
}

// T-GRAPH-DECAY-001.6: RTM NFR-7.5.5
#[test]
fn decay_function_default_is_power_law() {
    let _guard = TEST_MUTEX.lock();
    let _env = clear_decay_env();
    let env = isolated_env();

    assert_eq!(env.cfg.decay_function(), "power-law");
}

// T-GRAPH-DECAY-001.2-4: RTM NFR-7.5.x
#[test]
fn get_config_summary_includes_decay_config() {
    let _guard = TEST_MUTEX.lock();
    let _env = clear_decay_env();
    let env = isolated_env();

    let summary = config_summary(&env);

    assert!(summary.contains("Decay Config:"));
    assert!(summary.contains("Grace Days (Sequential):"));
    assert!(summary.contains("Grace Days (Workflows):"));
    assert!(summary.contains("Grace Days (Default):"));
    assert!(summary.contains("Half-Life Days:"));
    assert!(summary.contains("Function:"));
}

// T-GRAPH-DECAY-001.2-4: RTM NFR-7.5.x
#[test]
fn get_config_summary_shows_actual_values() {
    let _guard = TEST_MUTEX.lock();
    let _env = apply_decay_env(&[
        ("MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL", Some("9")),
        ("MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS", Some("18")),
        ("MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT", Some("33")),
        ("MAENIFOLD_DECAY_HALF_LIFE_DAYS", Some("45")),
        ("MAENIFOLD_DECAY_FUNCTION", Some("exponential")),
    ]);
    let env = isolated_env();

    let summary = config_summary(&env);

    assert!(summary.contains("Grace Days (Sequential): 9"));
    assert!(summary.contains("Grace Days (Workflows): 18"));
    assert!(summary.contains("Grace Days (Default): 33"));
    assert!(summary.contains("Half-Life Days: 45"));
    assert!(summary.contains("Function: exponential"));
}

// T-GRAPH-DECAY-001.2-4: RTM NFR-7.5.x
#[test]
fn compute_decay_weight_sequential_path_uses_config_grace_period() {
    let _guard = TEST_MUTEX.lock();
    let _env = clear_decay_env();
    let env = isolated_env();
    let last_accessed = rfc3339_days_ago(env.cfg.decay_grace_days_sequential());

    let weight = memory::compute_decay_weight(
        "memory://thinking/sequential/2026/03/session-123",
        Some(&last_accessed),
        &env.cfg,
        None,
    );

    assert_approx_eq(weight, 1.0, 0.0001);
}

// T-GRAPH-DECAY-001.2-4: RTM NFR-7.5.x
#[test]
fn compute_decay_weight_workflow_path_uses_config_grace_period() {
    let _guard = TEST_MUTEX.lock();
    let _env = clear_decay_env();
    let env = isolated_env();
    let last_accessed = rfc3339_days_ago(env.cfg.decay_grace_days_workflows());

    let weight = memory::compute_decay_weight(
        "memory://thinking/workflows/2026/03/workflow-abc",
        Some(&last_accessed),
        &env.cfg,
        None,
    );

    assert_approx_eq(weight, 1.0, 0.0001);
}

// T-GRAPH-DECAY-001.2-4: RTM NFR-7.5.x
#[test]
fn compute_decay_weight_default_path_uses_config_grace_period() {
    let _guard = TEST_MUTEX.lock();
    let _env = clear_decay_env();
    let env = isolated_env();
    let last_accessed = rfc3339_days_ago(env.cfg.decay_grace_days_default());

    let weight = memory::compute_decay_weight(
        "memory://notes/my-note",
        Some(&last_accessed),
        &env.cfg,
        None,
    );

    assert_approx_eq(weight, 1.0, 0.0001);
}

// T-GRAPH-DECAY-001.4: RTM NFR-7.5.3
#[test]
fn compute_decay_weight_uses_config_half_life() {
    let _guard = TEST_MUTEX.lock();
    let _env = clear_decay_env();
    let env = isolated_env();
    let age_days = env.cfg.decay_grace_days_default() + env.cfg.decay_half_life_days();
    let last_accessed = rfc3339_days_ago(age_days);

    let weight = memory::compute_decay_weight(
        "memory://notes/my-note",
        Some(&last_accessed),
        &env.cfg,
        None,
    );

    assert_approx_eq(weight, 0.1826, 0.01);
}

// T-GRAPH-DECAY-001.6: RTM NFR-7.5.5
#[test]
fn compute_decay_weight_exponential_function_applies_correctly() {
    let _guard = TEST_MUTEX.lock();
    let _env = apply_decay_env(&[("MAENIFOLD_DECAY_FUNCTION", Some("exponential"))]);
    let env = isolated_env();
    let age_days = env.cfg.decay_grace_days_default() + env.cfg.decay_half_life_days();
    let last_accessed = rfc3339_days_ago(age_days);

    let weight = memory::compute_decay_weight(
        "memory://notes/my-note",
        Some(&last_accessed),
        &env.cfg,
        None,
    );

    assert_approx_eq(weight, 0.5, 0.0001);
}

// T-GRAPH-DECAY-001.6: RTM NFR-7.5.5
#[test]
fn compute_decay_weight_decay_function_selection_changes_result_curve() {
    let _guard = TEST_MUTEX.lock();
    let _env = clear_decay_env();
    let env = isolated_env();
    let age_days = env.cfg.decay_grace_days_default() + env.cfg.decay_half_life_days();
    let last_accessed = rfc3339_days_ago(age_days);

    let power_law = memory::compute_decay_weight(
        "memory://notes/my-note",
        Some(&last_accessed),
        &env.cfg,
        None,
    );

    // SAFETY: all tests in this file hold TEST_MUTEX for their full duration,
    // so process-wide environment mutations are serialized.
    unsafe { std::env::set_var("MAENIFOLD_DECAY_FUNCTION", "exponential") };

    let exponential = memory::compute_decay_weight(
        "memory://notes/my-note",
        Some(&last_accessed),
        &env.cfg,
        None,
    );

    assert!(
        (power_law - exponential).abs() > 0.01,
        "power-law and exponential decay should diverge: power_law={power_law}, exponential={exponential}"
    );
}

// T-GRAPH-DECAY-001.2-4: RTM NFR-7.5.x
#[test]
fn invalid_integer_decay_env_vars_fall_back_to_defaults() {
    let _guard = TEST_MUTEX.lock();
    let _env = apply_decay_env(&[
        ("MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL", Some("invalid")),
        ("MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS", Some("invalid")),
        ("MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT", Some("invalid")),
        ("MAENIFOLD_DECAY_HALF_LIFE_DAYS", Some("invalid")),
    ]);
    let env = isolated_env();
    let summary = config_summary(&env);

    assert_eq!(env.cfg.decay_grace_days_sequential(), 7);
    assert_eq!(env.cfg.decay_grace_days_workflows(), 14);
    assert_eq!(env.cfg.decay_grace_days_default(), 28);
    assert_eq!(env.cfg.decay_half_life_days(), 30);
    assert!(summary.contains("Grace Days (Sequential): 7"));
    assert!(summary.contains("Grace Days (Workflows): 14"));
    assert!(summary.contains("Grace Days (Default): 28"));
    assert!(summary.contains("Half-Life Days: 30"));
}

// T-GRAPH-DECAY-001.6: RTM NFR-7.5.5
#[test]
fn empty_decay_function_falls_back_to_power_law() {
    let _guard = TEST_MUTEX.lock();
    let _env = apply_decay_env(&[("MAENIFOLD_DECAY_FUNCTION", Some("   "))]);
    let env = isolated_env();
    let summary = config_summary(&env);
    let age_days = env.cfg.decay_grace_days_default() + env.cfg.decay_half_life_days();
    let last_accessed = rfc3339_days_ago(age_days);

    let weight = memory::compute_decay_weight(
        "memory://notes/my-note",
        Some(&last_accessed),
        &env.cfg,
        None,
    );

    assert_eq!(env.cfg.decay_function(), "power-law");
    assert!(summary.contains("Function: power-law"));
    assert_approx_eq(weight, 0.1826, 0.01);
}

// T-GRAPH-DECAY-001.2-4: RTM NFR-7.5.x
#[test]
fn decay_config_all_values_are_positive() {
    let _guard = TEST_MUTEX.lock();
    let _env = clear_decay_env();
    let env = isolated_env();

    assert!(env.cfg.decay_grace_days_sequential() > 0);
    assert!(env.cfg.decay_grace_days_workflows() > 0);
    assert!(env.cfg.decay_grace_days_default() > 0);
    assert!(env.cfg.decay_half_life_days() > 0);
}
