use chrono::{Duration, Utc};
use maenifold_core::{Config, memory::compute_decay_weight};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

struct TestEnv {
    _tmp: TempDir,
    cfg: Config,
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
    fn with_values(
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

        // SAFETY: all tests in this file hold TEST_MUTEX for their full duration,
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

    fn exponential_defaults() -> Self {
        Self::with_values("exponential", 30, 7, 14, 28)
    }

    fn power_law_defaults() -> Self {
        Self::with_values("power-law", 30, 7, 14, 28)
    }
}

impl Drop for DecayEnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.previous {
            if let Some(value) = value {
                // SAFETY: all tests in this file hold TEST_MUTEX for their full duration,
                // so these process-wide environment mutations are serialized.
                unsafe {
                    std::env::set_var(key, value);
                }
            } else {
                // SAFETY: all tests in this file hold TEST_MUTEX for their full duration,
                // so these process-wide environment mutations are serialized.
                unsafe {
                    std::env::remove_var(key);
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

fn rfc3339_days_ago(days_ago: i64) -> String {
    (Utc::now() - Duration::days(days_ago)).to_rfc3339()
}

fn weight_for(cfg: &Config, file_path: &str, days_ago: Option<i64>, status: Option<&str>) -> f64 {
    let last_accessed = days_ago.map(rfc3339_days_ago);
    compute_decay_weight(file_path, last_accessed.as_deref(), cfg, status)
}

fn assert_approx_eq(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected} ± {tolerance}, got {actual}"
    );
}

// These tests port the C# DecayCalculator coverage scenarios onto Rust's single
// `compute_decay_weight(file_path, last_accessed, config, status)` API surface.
// Rust does not expose the C# overload pair or a separate `created` timestamp,
// so overload-only cases are represented through equivalent single-entrypoint
// scenarios and `last_accessed`-driven reference dates.

// T-7.8: RTM FR-7.8
#[test]
fn validated_status_returns_one_when_new() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let weight = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/validated-new",
        Some(1),
        Some("validated"),
    );

    assert_eq!(weight, 1.0);
}

// T-7.8: RTM FR-7.8
#[test]
fn validated_status_returns_one_when_very_old() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let weight = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/validated-old",
        Some(365),
        Some("validated"),
    );

    assert_eq!(weight, 1.0);
}

// T-7.8: RTM FR-7.8
#[test]
fn validated_status_is_case_insensitive_uppercase() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let weight = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/validated-upper",
        Some(90),
        Some("VALIDATED"),
    );

    assert_eq!(weight, 1.0);
}

// T-7.8: RTM FR-7.8
#[test]
fn validated_status_is_case_insensitive_mixed_case() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let weight = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/validated-mixed",
        Some(60),
        Some("Validated"),
    );

    assert_eq!(weight, 1.0);
}

// T-7.8: RTM FR-7.8
#[test]
fn invalidated_status_within_grace_period_returns_one() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let weight = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/invalidated-grace",
        Some(3),
        Some("invalidated"),
    );

    assert_eq!(weight, 1.0);
}

// T-7.8: RTM FR-7.8
#[test]
fn invalidated_status_past_grace_period_decays_aggressively() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let weight = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/invalidated-half-life",
        Some(7 + 14),
        Some("invalidated"),
    );

    assert_approx_eq(weight, 0.5, 0.01);
}

// T-7.8: RTM FR-7.8
#[test]
fn invalidated_status_is_case_insensitive() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let lower = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/invalidated-lower",
        Some(30),
        Some("invalidated"),
    );
    let upper = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/invalidated-upper",
        Some(30),
        Some("INVALIDATED"),
    );

    assert_approx_eq(upper, lower, 1e-9);
}

// T-7.8: RTM FR-7.8
#[test]
fn invalidated_status_decays_faster_than_active() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let invalidated = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/invalidated-vs-active",
        Some(30),
        Some("invalidated"),
    );
    let active = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/active-vs-invalidated",
        Some(30),
        Some("active"),
    );

    assert!(
        invalidated < active,
        "expected invalidated={invalidated} to decay faster than active={active}"
    );
}

// T-7.8: RTM FR-7.8
#[test]
fn active_status_within_grace_period_returns_one() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let weight = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/active-grace",
        Some(10),
        Some("active"),
    );

    assert_eq!(weight, 1.0);
}

// T-7.8: RTM FR-7.8
#[test]
fn active_status_past_grace_period_decays_with_thirty_day_half_life() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let weight = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/active-half-life",
        Some(14 + 30),
        Some("active"),
    );

    assert_approx_eq(weight, 0.5, 0.01);
}

// T-7.8: RTM FR-7.8
#[test]
fn active_status_is_case_insensitive() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let lower = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/active-lower",
        Some(50),
        Some("active"),
    );
    let upper = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/active-upper",
        Some(50),
        Some("ACTIVE"),
    );

    assert_approx_eq(upper, lower, 1e-9);
}

// T-7.8: RTM FR-7.8
#[test]
fn refined_status_within_grace_period_returns_one() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let weight = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/refined-grace",
        Some(5),
        Some("refined"),
    );

    assert_eq!(weight, 1.0);
}

// T-7.8: RTM FR-7.8
#[test]
fn refined_status_uses_same_parameters_as_active() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let refined = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/refined-vs-active",
        Some(60),
        Some("refined"),
    );
    let active = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/active-vs-refined",
        Some(60),
        Some("active"),
    );

    assert_approx_eq(refined, active, 1e-9);
}

// T-7.8: RTM FR-7.8
#[test]
fn refined_status_is_case_insensitive() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let lower = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/refined-lower",
        Some(50),
        Some("refined"),
    );
    let upper = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/refined-upper",
        Some(50),
        Some("REFINED"),
    );

    assert_approx_eq(upper, lower, 1e-9);
}

// T-7.8: RTM FR-7.8
#[test]
fn unknown_status_falls_back_to_path_defaults() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let unknown = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/unknown-status",
        Some(60),
        Some("unknown-garbage"),
    );
    let absent = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/absent-status",
        Some(60),
        None,
    );

    assert_approx_eq(unknown, absent, 1e-9);
}

// T-7.8: RTM FR-7.8
#[test]
fn empty_status_falls_back_to_path_defaults() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let empty = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/empty-status",
        Some(60),
        Some(""),
    );
    let absent = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/empty-status-baseline",
        Some(60),
        None,
    );

    assert_approx_eq(empty, absent, 1e-9);
}

// T-7.8: RTM FR-7.8
#[test]
fn absent_status_uses_path_defaults() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let absent = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/no-status",
        Some(60),
        None,
    );
    let explicit_default = weight_for(
        &env.cfg,
        "memory://notes/2026/03/default-path",
        Some(60),
        None,
    );

    assert_approx_eq(absent, explicit_default, 1e-9);
}

// T-7.8: RTM FR-7.8
#[test]
fn whitespace_status_falls_back_to_path_defaults() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let whitespace = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/whitespace-status",
        Some(60),
        Some("   "),
    );
    let absent = weight_for(
        &env.cfg,
        "memory://assumptions/2026/03/whitespace-status-baseline",
        Some(60),
        None,
    );

    assert_approx_eq(whitespace, absent, 1e-9);
}

// T-7.6.1: RTM NFR-7.6.1
#[test]
fn missing_last_accessed_defaults_to_one() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let weight = compute_decay_weight("memory://notes/no-accessed", None, &env.cfg, None);

    assert_eq!(weight, 1.0);
}

// T-7.6.1: RTM NFR-7.6.1
#[test]
fn recent_last_accessed_keeps_weight_at_one() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let weight = weight_for(&env.cfg, "memory://notes/recent-access", Some(1), None);

    assert_eq!(weight, 1.0);
}

// T-7.6.1: RTM NFR-7.6.1
#[test]
fn recent_last_accessed_produces_higher_weight_than_old_last_accessed() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let old_weight = weight_for(&env.cfg, "memory://notes/older-access", Some(60), None);
    let recent_weight = weight_for(&env.cfg, "memory://notes/recent-access", Some(1), None);

    assert!(
        recent_weight > old_weight,
        "expected recent_weight={recent_weight} to exceed old_weight={old_weight}"
    );
}

// T-7.5.2: RTM NFR-7.5.2
#[test]
fn empty_path_uses_default_grace_period() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let weight = weight_for(&env.cfg, "", Some(28), None);

    assert_approx_eq(weight, 1.0, 1e-9);
}

// T-7.5.1: RTM NFR-7.5.1
#[test]
fn windows_backslash_sequential_path_uses_sequential_grace_period() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let windows_path = r"C:\Users\user\maenifold\memory\thinking\sequential\session.md";
    let weight = weight_for(&env.cfg, windows_path, Some(7), None);

    assert_approx_eq(weight, 1.0, 1e-9);
}

// T-7.5.1a: RTM NFR-7.5.1
#[test]
fn thinking_workflow_path_uses_workflow_grace_period() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let weight = weight_for(
        &env.cfg,
        "memory://thinking/workflow/2026/03/workflow-note",
        Some(14),
        None,
    );

    assert_approx_eq(weight, 1.0, 1e-9);
}

// T-7.5.2: RTM NFR-7.5.2
#[test]
fn regular_path_uses_default_grace_period() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let weight = weight_for(&env.cfg, "memory://notes/my-note", Some(28), None);

    assert_approx_eq(weight, 1.0, 1e-9);
}

// T-7.5.1: RTM NFR-7.5.1
#[test]
fn sequential_path_has_shorter_grace_than_default_path() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::exponential_defaults();
    let env = isolated_env();

    let sequential = weight_for(
        &env.cfg,
        "memory://thinking/sequential/2026/03/session",
        Some(10),
        None,
    );
    let regular = weight_for(&env.cfg, "memory://notes/reference-note", Some(10), None);

    assert!(
        sequential < regular,
        "expected sequential={sequential} to decay faster than regular={regular}"
    );
    assert_approx_eq(regular, 1.0, 1e-9);
}

// T-7.5.5: RTM NFR-7.5.5
#[test]
fn power_law_within_grace_period_returns_one() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::power_law_defaults();
    let env = isolated_env();

    let weight = weight_for(
        &env.cfg,
        "memory://notes/power-law-within-grace",
        Some(5),
        None,
    );

    assert_eq!(weight, 1.0);
}

// T-7.5.5: RTM NFR-7.5.5
#[test]
fn power_law_at_grace_boundary_returns_one() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::power_law_defaults();
    let env = isolated_env();

    let weight = weight_for(
        &env.cfg,
        "memory://thinking/workflow/2026/03/power-law-boundary",
        Some(14),
        None,
    );

    assert_eq!(weight, 1.0);
}

// T-7.5.5: RTM NFR-7.5.5
#[test]
fn power_law_past_grace_period_calculates_effective_age_decay() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::power_law_defaults();
    let env = isolated_env();

    let weight = weight_for(
        &env.cfg,
        "memory://thinking/workflow/2026/03/power-law-decay",
        Some(14 + 4),
        None,
    );

    assert_approx_eq(weight, 0.5, 1e-9);
}

// T-7.5.5: RTM NFR-7.5.5
#[test]
fn power_law_produces_different_result_from_exponential() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let _power_law = DecayEnvGuard::power_law_defaults();
    let power_law = weight_for(
        &env.cfg,
        "memory://thinking/workflow/2026/03/power-law-vs-exponential",
        Some(14 + 60),
        None,
    );
    drop(_power_law);

    let _exponential = DecayEnvGuard::exponential_defaults();
    let exponential = weight_for(
        &env.cfg,
        "memory://thinking/workflow/2026/03/power-law-vs-exponential",
        Some(14 + 60),
        None,
    );

    assert!(
        (power_law - exponential).abs() > 0.01,
        "expected power_law={power_law} and exponential={exponential} to diverge"
    );
}

// T-7.5.5: RTM NFR-7.5.5
#[test]
fn power_law_function_string_is_case_insensitive() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let _lower = DecayEnvGuard::with_values("power-law", 30, 7, 14, 28);
    let lower = weight_for(
        &env.cfg,
        "memory://thinking/workflow/2026/03/power-law-case",
        Some(18),
        None,
    );
    drop(_lower);

    let _upper = DecayEnvGuard::with_values("POWER-LAW", 30, 7, 14, 28);
    let upper = weight_for(
        &env.cfg,
        "memory://thinking/workflow/2026/03/power-law-case",
        Some(18),
        None,
    );
    drop(_upper);

    let _mixed = DecayEnvGuard::with_values("Power-Law", 30, 7, 14, 28);
    let mixed = weight_for(
        &env.cfg,
        "memory://thinking/workflow/2026/03/power-law-case",
        Some(18),
        None,
    );

    assert_approx_eq(upper, lower, 1e-9);
    assert_approx_eq(mixed, lower, 1e-9);
}

// T-7.5.5: RTM NFR-7.5.5
#[test]
fn power_law_weight_is_bounded_between_zero_and_one() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::power_law_defaults();
    let env = isolated_env();

    for age_days in [0, 1, 14, 15, 30, 100, 365, 3650] {
        let weight = weight_for(
            &env.cfg,
            "memory://thinking/workflow/2026/03/bounded",
            Some(age_days),
            None,
        );
        assert!(
            (0.0..=1.0).contains(&weight),
            "expected bounded weight for age_days={age_days}, got {weight}"
        );
    }
}
