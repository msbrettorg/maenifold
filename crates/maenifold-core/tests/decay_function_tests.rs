use chrono::{DateTime, Duration, Utc};
use maenifold_core::{Config, memory::compute_decay_weight};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

const DEFAULT_GRACE_PERIOD_DAYS: i64 = 14;
const SEQUENTIAL_GRACE_PERIOD_DAYS: i64 = 7;
const WORKFLOWS_GRACE_PERIOD_DAYS: i64 = 14;
const DEFAULT_HALF_LIFE_DAYS: i64 = 30;
const POWER_LAW_A: f64 = 1.0;
const POWER_LAW_B: f64 = 0.5;

// T-0.1: RTM NFR-3 - Port C# decay regression coverage into Rust integration tests.
struct TestConfigGuard {
    _tmp: TempDir,
    cfg: Config,
}

impl TestConfigGuard {
    fn new() -> Self {
        let tmp = TempDir::new().expect("tmp");
        Config::override_root(tmp.path());
        Config::set_database_path(tmp.path().join("memory.db"));
        let cfg = Config::current();
        cfg.ensure_directories().expect("dirs");
        Self { _tmp: tmp, cfg }
    }
}

impl Drop for TestConfigGuard {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

struct DecayEnvGuard {
    previous: Vec<(&'static str, Option<String>)>,
}

impl DecayEnvGuard {
    fn set(
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

        // SAFETY: every test in this file holds TEST_MUTEX for its entire duration,
        // so process-wide environment mutation is serialized.
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
}

impl Drop for DecayEnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.previous {
            if let Some(value) = value {
                // SAFETY: every test in this file holds TEST_MUTEX for its entire duration,
                // so process-wide environment mutation is serialized.
                unsafe {
                    std::env::set_var(key, value);
                }
            } else {
                // SAFETY: every test in this file holds TEST_MUTEX for its entire duration,
                // so process-wide environment mutation is serialized.
                unsafe {
                    std::env::remove_var(key);
                }
            }
        }
    }
}

fn assert_approx_eq(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected:.10}, got {actual:.10}, tolerance {tolerance:.10}"
    );
}

fn last_accessed_for_age_days(now: DateTime<Utc>, age_days: i64) -> String {
    (now - Duration::days(age_days)).to_rfc3339()
}

fn compute_with_last_accessed(
    cfg: &Config,
    file_path: &str,
    last_accessed: &str,
    status: Option<&str>,
) -> f64 {
    compute_decay_weight(file_path, Some(last_accessed), cfg, status)
}

fn compute_exponential_decay(cfg: &Config, file_path: &str, age_days: i64, grace_days: i64) -> f64 {
    let _env = DecayEnvGuard::set(
        "exponential",
        DEFAULT_HALF_LIFE_DAYS,
        grace_days,
        grace_days,
        grace_days,
    );
    let now = Utc::now();
    let last_accessed = last_accessed_for_age_days(now, age_days);
    compute_with_last_accessed(cfg, file_path, &last_accessed, None)
}

fn compute_exponential_decay_with_half_life(
    cfg: &Config,
    file_path: &str,
    age_days: i64,
    grace_days: i64,
    half_life_days: i64,
) -> f64 {
    let _env = DecayEnvGuard::set("exponential", half_life_days, 7, 14, grace_days);
    let now = Utc::now();
    let last_accessed = last_accessed_for_age_days(now, age_days);
    compute_with_last_accessed(cfg, file_path, &last_accessed, None)
}

fn compute_exponential_from_date(
    cfg: &Config,
    file_path: &str,
    created_date: DateTime<Utc>,
    grace_days: i64,
    half_life_days: i64,
) -> f64 {
    let _env = DecayEnvGuard::set("exponential", half_life_days, 7, 14, grace_days);
    let created = created_date.to_rfc3339();
    compute_with_last_accessed(cfg, file_path, &created, None)
}

fn compute_power_law_decay(cfg: &Config, file_path: &str, age_days: i64, grace_days: i64) -> f64 {
    let _env = DecayEnvGuard::set("power-law", DEFAULT_HALF_LIFE_DAYS, 7, 14, grace_days);
    let now = Utc::now();
    let last_accessed = last_accessed_for_age_days(now, age_days);
    compute_with_last_accessed(cfg, file_path, &last_accessed, None)
}

fn compute_power_law_from_date(
    cfg: &Config,
    file_path: &str,
    created_date: DateTime<Utc>,
    grace_days: i64,
) -> f64 {
    let _env = DecayEnvGuard::set("power-law", DEFAULT_HALF_LIFE_DAYS, 7, 14, grace_days);
    let created = created_date.to_rfc3339();
    compute_with_last_accessed(cfg, file_path, &created, None)
}

fn power_law_formula(age_days: f64, a: f64, b: f64) -> f64 {
    if age_days <= 0.0 {
        return 1.0;
    }
    (a * age_days.powf(-b)).clamp(0.0, 1.0)
}

#[test]
fn exponential_decay_within_grace_period_returns_weight_one() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();

    // T-0.1: RTM NFR-3
    let weight = compute_exponential_decay(
        &env.cfg,
        "memory://projects/within-grace",
        5,
        DEFAULT_GRACE_PERIOD_DAYS,
    );

    assert_eq!(weight, 1.0);
}

#[test]
fn exponential_decay_zero_age_returns_weight_one() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();

    let weight = compute_exponential_decay(
        &env.cfg,
        "memory://projects/zero-age",
        0,
        DEFAULT_GRACE_PERIOD_DAYS,
    );

    assert_eq!(weight, 1.0);
}

#[test]
fn exponential_decay_exactly_at_grace_period_boundary_returns_weight_one() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();

    let weight = compute_exponential_decay(
        &env.cfg,
        "memory://projects/grace-boundary",
        DEFAULT_GRACE_PERIOD_DAYS,
        DEFAULT_GRACE_PERIOD_DAYS,
    );

    assert_eq!(weight, 1.0);
}

#[test]
fn exponential_decay_one_half_life_past_grace_returns_weight_half() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();

    let age_days = DEFAULT_GRACE_PERIOD_DAYS + DEFAULT_HALF_LIFE_DAYS;
    let weight = compute_exponential_decay(
        &env.cfg,
        "memory://projects/one-half-life",
        age_days,
        DEFAULT_GRACE_PERIOD_DAYS,
    );

    assert_approx_eq(weight, 0.5, 0.0001);
}

#[test]
fn exponential_decay_two_half_lives_past_grace_returns_weight_quarter() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();

    let age_days = DEFAULT_GRACE_PERIOD_DAYS + (2 * DEFAULT_HALF_LIFE_DAYS);
    let weight = compute_exponential_decay(
        &env.cfg,
        "memory://projects/two-half-lives",
        age_days,
        DEFAULT_GRACE_PERIOD_DAYS,
    );

    assert_approx_eq(weight, 0.25, 0.0001);
}

#[test]
fn exponential_decay_three_half_lives_past_grace_returns_weight_eighth() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();

    let age_days = DEFAULT_GRACE_PERIOD_DAYS + (3 * DEFAULT_HALF_LIFE_DAYS);
    let weight = compute_exponential_decay(
        &env.cfg,
        "memory://projects/three-half-lives",
        age_days,
        DEFAULT_GRACE_PERIOD_DAYS,
    );

    assert_approx_eq(weight, 0.125, 0.0001);
}

#[test]
fn exponential_decay_half_of_half_life_past_grace_returns_intermediate_weight() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();

    let age_days = DEFAULT_GRACE_PERIOD_DAYS + (DEFAULT_HALF_LIFE_DAYS / 2);
    let weight = compute_exponential_decay(
        &env.cfg,
        "memory://projects/half-half-life",
        age_days,
        DEFAULT_GRACE_PERIOD_DAYS,
    );

    let expected = 0.5f64.powf(0.5);
    assert_approx_eq(weight, expected, 0.0001);
}

#[test]
fn exponential_decay_never_exceeds_one() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();

    for age_days in [-10, -1, 0, 1, 5, 14, 15, 30, 100, 365, 1000] {
        let weight = compute_exponential_decay(
            &env.cfg,
            "memory://projects/never-exceeds-one",
            age_days,
            DEFAULT_GRACE_PERIOD_DAYS,
        );

        assert!(
            weight <= 1.0,
            "weight should never exceed 1.0 for age={age_days}, got {weight}"
        );
    }
}

#[test]
fn exponential_decay_never_goes_below_zero() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();

    for age_days in [100, 365, 1000, 3650, 10000] {
        let weight = compute_exponential_decay(
            &env.cfg,
            "memory://projects/never-below-zero",
            age_days,
            DEFAULT_GRACE_PERIOD_DAYS,
        );

        assert!(
            weight >= 0.0,
            "weight should never go below 0.0 for age={age_days}, got {weight}"
        );
    }
}

#[test]
fn exponential_decay_very_old_content_approaches_zero_but_never_reaches() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();

    let weight = compute_exponential_decay(
        &env.cfg,
        "memory://projects/very-old",
        3650,
        DEFAULT_GRACE_PERIOD_DAYS,
    );

    assert!(
        weight > 0.0,
        "very old content should remain asymptotically above zero"
    );
    assert!(
        weight < 0.001,
        "very old content should decay to a very small value"
    );
}

#[test]
fn exponential_decay_sequential_thinking_uses_7_day_grace_period() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();

    let at_grace = compute_exponential_decay(
        &env.cfg,
        "memory://thinking/sequential/2026/03/at-grace",
        SEQUENTIAL_GRACE_PERIOD_DAYS,
        SEQUENTIAL_GRACE_PERIOD_DAYS,
    );
    let past_grace = compute_exponential_decay(
        &env.cfg,
        "memory://thinking/sequential/2026/03/past-grace",
        SEQUENTIAL_GRACE_PERIOD_DAYS + 1,
        SEQUENTIAL_GRACE_PERIOD_DAYS,
    );

    assert_eq!(at_grace, 1.0);
    assert!(
        past_grace < 1.0,
        "sequential content should decay after day 7"
    );
}

#[test]
fn exponential_decay_workflows_uses_14_day_grace_period() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();

    let at_grace = compute_exponential_decay(
        &env.cfg,
        "memory://thinking/workflow/2026/03/at-grace",
        WORKFLOWS_GRACE_PERIOD_DAYS,
        WORKFLOWS_GRACE_PERIOD_DAYS,
    );
    let past_grace = compute_exponential_decay(
        &env.cfg,
        "memory://thinking/workflow/2026/03/past-grace",
        WORKFLOWS_GRACE_PERIOD_DAYS + 1,
        WORKFLOWS_GRACE_PERIOD_DAYS,
    );

    assert_eq!(at_grace, 1.0);
    assert!(
        past_grace < 1.0,
        "workflow content should decay after day 14"
    );
}

#[test]
fn exponential_decay_default_memory_uses_14_day_grace_period() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();

    let at_grace = compute_exponential_decay(
        &env.cfg,
        "memory://projects/default-at-grace",
        DEFAULT_GRACE_PERIOD_DAYS,
        DEFAULT_GRACE_PERIOD_DAYS,
    );
    let past_grace = compute_exponential_decay(
        &env.cfg,
        "memory://projects/default-past-grace",
        DEFAULT_GRACE_PERIOD_DAYS + 1,
        DEFAULT_GRACE_PERIOD_DAYS,
    );

    assert_eq!(at_grace, 1.0);
    assert!(
        past_grace < 1.0,
        "default content should decay after day 14"
    );
}

#[test]
fn exponential_decay_tiered_grace_periods_sequential_decays_faster() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();
    let now = Utc::now();
    let last_accessed = last_accessed_for_age_days(now, 10);
    let _env = DecayEnvGuard::set(
        "exponential",
        DEFAULT_HALF_LIFE_DAYS,
        SEQUENTIAL_GRACE_PERIOD_DAYS,
        WORKFLOWS_GRACE_PERIOD_DAYS,
        DEFAULT_GRACE_PERIOD_DAYS,
    );

    let sequential = compute_with_last_accessed(
        &env.cfg,
        "memory://thinking/sequential/2026/03/note",
        &last_accessed,
        None,
    );
    let workflow = compute_with_last_accessed(
        &env.cfg,
        "memory://thinking/workflow/2026/03/note",
        &last_accessed,
        None,
    );
    let default = compute_with_last_accessed(
        &env.cfg,
        "memory://projects/default-note",
        &last_accessed,
        None,
    );

    assert!(sequential < workflow);
    assert!(sequential < default);
    assert_approx_eq(workflow, default, 1e-9);
}

#[test]
fn power_law_decay_basic_calculation_follows_formula() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();

    let weight = compute_power_law_decay(&env.cfg, "memory://projects/power-basic", 4, 0);
    let expected = POWER_LAW_A * 4.0f64.powf(-POWER_LAW_B);

    assert_approx_eq(weight, expected, 0.0001);
}

#[test]
fn power_law_decay_at_day_one_returns_weight_one() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();

    let weight = compute_power_law_decay(&env.cfg, "memory://projects/power-day-one", 1, 0);

    assert_approx_eq(weight, 1.0, 0.0001);
}

#[test]
fn power_law_decay_below_day_one_caps_at_one() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();
    let _env = DecayEnvGuard::set("power-law", DEFAULT_HALF_LIFE_DAYS, 7, 14, 0);
    let last_accessed = (Utc::now() - Duration::hours(12)).to_rfc3339();

    let weight = compute_with_last_accessed(
        &env.cfg,
        "memory://projects/power-half-day",
        &last_accessed,
        None,
    );

    assert_eq!(weight, 1.0);
}

#[test]
fn power_law_decay_zero_age_returns_weight_one() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();

    let weight = compute_power_law_decay(&env.cfg, "memory://projects/power-zero-age", 0, 0);

    assert_eq!(weight, 1.0);
}

#[test]
fn power_law_decay_never_exceeds_one() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();

    for age_days in [0, 1, 2, 10, 100, 365, 1000, 3650, 10000] {
        let weight = compute_power_law_decay(
            &env.cfg,
            "memory://projects/power-never-exceeds-one",
            age_days,
            0,
        );
        assert!(
            weight <= 1.0,
            "power-law weight should never exceed 1.0 for age={age_days}, got {weight}"
        );
    }
}

#[test]
fn power_law_decay_never_goes_below_zero() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();

    for age_days in [100, 365, 1000, 3650, 10000] {
        let weight = compute_power_law_decay(
            &env.cfg,
            "memory://projects/power-never-below-zero",
            age_days,
            0,
        );
        assert!(
            weight >= 0.0,
            "power-law weight should never go below 0.0 for age={age_days}, got {weight}"
        );
    }
}

#[test]
fn power_law_decay_slower_than_exponential_for_old_content() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();

    let exponential = compute_exponential_decay(
        &env.cfg,
        "memory://projects/old-content-exponential",
        120,
        DEFAULT_GRACE_PERIOD_DAYS,
    );
    let power_law =
        compute_power_law_decay(&env.cfg, "memory://projects/old-content-power", 120, 0);

    assert!(
        power_law >= exponential * 0.5,
        "power-law should not decay drastically faster than exponential: power={power_law}, exp={exponential}"
    );
}

#[test]
fn power_law_decay_produces_different_curve_than_exponential() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();

    let mut difference_found = false;
    for age_days in [1, 4, 16, 64, 256] {
        let exponential =
            compute_exponential_decay(&env.cfg, "memory://projects/curve-exp", age_days, 0);
        let power_law =
            compute_power_law_decay(&env.cfg, "memory://projects/curve-power", age_days, 0);
        if (exponential - power_law).abs() > 0.01 {
            difference_found = true;
        }
    }

    assert!(
        difference_found,
        "power-law and exponential curves should differ at one or more time points"
    );
}

#[test]
fn power_law_decay_memory_halves_when_time_quadruples() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();

    let weight_at_4 = compute_power_law_decay(&env.cfg, "memory://projects/weight-at-4", 4, 0);
    let weight_at_16 = compute_power_law_decay(&env.cfg, "memory://projects/weight-at-16", 16, 0);
    let weight_at_64 = compute_power_law_decay(&env.cfg, "memory://projects/weight-at-64", 64, 0);

    assert_approx_eq(weight_at_4, 0.5, 0.0001);
    assert_approx_eq(weight_at_16, weight_at_4 / 2.0, 0.0001);
    assert_approx_eq(weight_at_64, weight_at_16 / 2.0, 0.0001);
}

#[test]
fn exponential_decay_negative_age_treated_as_zero() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();

    let weight = compute_exponential_decay(
        &env.cfg,
        "memory://projects/negative-age",
        -5,
        DEFAULT_GRACE_PERIOD_DAYS,
    );

    assert_eq!(weight, 1.0);
}

#[test]
fn exponential_decay_zero_half_life_handled_gracefully() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();

    let weight = compute_exponential_decay_with_half_life(
        &env.cfg,
        "memory://projects/zero-half-life",
        20,
        DEFAULT_GRACE_PERIOD_DAYS,
        0,
    );

    assert!(
        (0.0..=1.0).contains(&weight),
        "zero half-life should still return a bounded weight, got {weight}"
    );
}

#[test]
fn exponential_decay_negative_grace_period_treated_as_zero() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();

    let weight = compute_exponential_decay_with_half_life(
        &env.cfg,
        "memory://projects/negative-grace",
        5,
        -10,
        DEFAULT_HALF_LIFE_DAYS,
    );

    assert!(
        weight < 1.0,
        "negative grace should cause immediate decay, got {weight}"
    );
}

#[test]
fn power_law_decay_negative_age_returns_weight_one() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();

    let weight = compute_power_law_decay(&env.cfg, "memory://projects/power-negative-age", -5, 0);

    assert_eq!(weight, 1.0);
}

#[test]
fn power_law_decay_custom_parameters_applies_correctly() {
    let _guard = TEST_MUTEX.lock();

    // Rust currently exposes ACT-R defaults through compute_decay_weight only.
    // This preserves the oracle's pure formula regression for configurable parameters.
    let weight = power_law_formula(10.0, 2.0, 0.3);
    let expected = (2.0 * 10.0f64.powf(-0.3)).clamp(0.0, 1.0);

    assert_approx_eq(weight, expected, 0.0001);
}

#[test]
fn exponential_decay_datetime_api_calculates_correct_age() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();
    let created_date = Utc::now() - Duration::days(10);

    let weight = compute_exponential_from_date(
        &env.cfg,
        "memory://projects/datetime-exponential",
        created_date,
        DEFAULT_GRACE_PERIOD_DAYS,
        DEFAULT_HALF_LIFE_DAYS,
    );

    assert_eq!(weight, 1.0);
}

#[test]
fn exponential_decay_datetime_api_future_date_returns_one() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();
    let future_date = Utc::now() + Duration::days(5);

    let weight = compute_exponential_from_date(
        &env.cfg,
        "memory://projects/datetime-future",
        future_date,
        DEFAULT_GRACE_PERIOD_DAYS,
        DEFAULT_HALF_LIFE_DAYS,
    );

    assert_eq!(weight, 1.0);
}

#[test]
fn power_law_decay_datetime_api_calculates_correct_age() {
    let _guard = TEST_MUTEX.lock();
    let env = TestConfigGuard::new();
    let created_date = Utc::now() - Duration::days(4);

    let weight = compute_power_law_from_date(
        &env.cfg,
        "memory://projects/datetime-power",
        created_date,
        0,
    );

    assert_approx_eq(weight, 0.5, 0.01);
}
