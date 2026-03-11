use chrono::{DateTime, Duration, Utc};
use maenifold_core::{Config, memory::compute_decay_weight};
use once_cell::sync::Lazy;
use parking_lot::{Mutex, MutexGuard};
use tempfile::TempDir;

pub static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub const DEFAULT_GRACE_PERIOD_DAYS: i64 = 14;
pub const SEQUENTIAL_GRACE_PERIOD_DAYS: i64 = 7;
pub const WORKFLOWS_GRACE_PERIOD_DAYS: i64 = 14;
pub const DEFAULT_HALF_LIFE_DAYS: i64 = 30;
pub const POWER_LAW_A: f64 = 1.0;
pub const POWER_LAW_B: f64 = 0.5;

// T-12.6: RTM NFR-1
pub fn acquire_test_lock() -> MutexGuard<'static, ()> {
    TEST_MUTEX.lock()
}

pub struct TestConfigGuard {
    _tmp: TempDir,
    pub cfg: Config,
}

impl TestConfigGuard {
    pub fn new() -> Self {
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

pub struct DecayEnvGuard {
    previous: Vec<(&'static str, Option<String>)>,
}

impl DecayEnvGuard {
    pub fn set(
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

        // SAFETY: every test acquires TEST_MUTEX for its full duration,
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
                // SAFETY: every test acquires TEST_MUTEX for its full duration,
                // so process-wide environment mutation is serialized.
                unsafe {
                    std::env::set_var(key, value);
                }
            } else {
                // SAFETY: every test acquires TEST_MUTEX for its full duration,
                // so process-wide environment mutation is serialized.
                unsafe {
                    std::env::remove_var(key);
                }
            }
        }
    }
}

pub fn assert_approx_eq(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected:.10}, got {actual:.10}, tolerance {tolerance:.10}"
    );
}

pub fn last_accessed_for_age_days(now: DateTime<Utc>, age_days: i64) -> String {
    (now - Duration::days(age_days)).to_rfc3339()
}

pub fn compute_with_last_accessed(
    cfg: &Config,
    file_path: &str,
    last_accessed: &str,
    status: Option<&str>,
) -> f64 {
    compute_decay_weight(file_path, Some(last_accessed), cfg, status)
}

pub fn compute_exponential_decay(
    cfg: &Config,
    file_path: &str,
    age_days: i64,
    grace_days: i64,
) -> f64 {
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

pub fn compute_exponential_decay_with_half_life(
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

pub fn compute_exponential_from_date(
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

pub fn compute_power_law_decay(
    cfg: &Config,
    file_path: &str,
    age_days: i64,
    grace_days: i64,
) -> f64 {
    let _env = DecayEnvGuard::set("power-law", DEFAULT_HALF_LIFE_DAYS, 7, 14, grace_days);
    let now = Utc::now();
    let last_accessed = last_accessed_for_age_days(now, age_days);
    compute_with_last_accessed(cfg, file_path, &last_accessed, None)
}

pub fn compute_power_law_from_date(
    cfg: &Config,
    file_path: &str,
    created_date: DateTime<Utc>,
    grace_days: i64,
) -> f64 {
    let _env = DecayEnvGuard::set("power-law", DEFAULT_HALF_LIFE_DAYS, 7, 14, grace_days);
    let created = created_date.to_rfc3339();
    compute_with_last_accessed(cfg, file_path, &created, None)
}

pub fn power_law_formula(age_days: f64, a: f64, b: f64) -> f64 {
    if age_days <= 0.0 {
        return 1.0;
    }
    (a * age_days.powf(-b)).clamp(0.0, 1.0)
}
