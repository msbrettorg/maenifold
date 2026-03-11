use crate::config::Config;
use chrono::Utc;

use super::parse_datetime;

// T-12.5: RTM NFR-1 isolates decay scoring rules from I/O helpers while keeping
// the shared C#-parity calculation logic intact for search and graph callers.
pub(crate) fn compute_decay_weight(
    file_path: &str,
    last_accessed: Option<&str>,
    config: &Config,
    status: Option<&str>,
) -> f64 {
    compute_decay_weight_with_created(file_path, last_accessed, None, config, status)
}

// T-14.1: RTM FR-21 restores C# parity by falling back from `last_accessed`
// to `created` instead of returning a neutral score when access metadata is missing.
pub(crate) fn compute_decay_weight_with_created(
    file_path: &str,
    last_accessed: Option<&str>,
    created: Option<&str>,
    config: &Config,
    status: Option<&str>,
) -> f64 {
    let mut grace_period_days = grace_period_for_path(file_path, config);
    let mut half_life_days = config.decay_half_life_days();
    let normalized_path = normalize_decay_path(file_path, config);
    let is_assumption_path = normalized_path.contains("/assumptions/");

    if is_assumption_path || normalized_path.contains("/chat/") {
        if let Some(status) = status {
            match status.trim().to_ascii_lowercase().as_str() {
                "validated" => return 1.0,
                "invalidated" => {
                    grace_period_days = 7;
                    half_life_days = 14;
                }
                "active" | "refined" => {
                    grace_period_days = 14;
                    half_life_days = 30;
                }
                _ => {}
            }
        }
    }

    let decay_function = if is_assumption_path {
        "exponential".to_string()
    } else {
        config.decay_function()
    };

    let Some(reference_date) = last_accessed
        .and_then(parse_datetime)
        .or_else(|| created.and_then(parse_datetime))
    else {
        return 1.0;
    };
    let age_days = (Utc::now() - reference_date).num_days();
    if decay_function.eq_ignore_ascii_case("power-law") {
        if age_days <= grace_period_days {
            return 1.0;
        }
        let effective_age = (age_days - grace_period_days).max(1) as f64;
        return effective_age.powf(-0.5).clamp(0.0, 1.0);
    }

    if age_days < 0 {
        return 1.0;
    }
    if age_days <= grace_period_days {
        return 1.0;
    }
    if half_life_days <= 0 {
        return 0.0;
    }
    let exponent = (age_days - grace_period_days) as f64 / half_life_days as f64;
    (0.5f64.powf(exponent)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::compute_decay_weight_with_created;
    use crate::config::Config;
    use chrono::{Duration, Utc};
    use tempfile::TempDir;

    struct ConfigResetGuard;

    impl Drop for ConfigResetGuard {
        fn drop(&mut self) {
            Config::reset_overrides();
        }
    }

    fn isolated_config() -> (TempDir, Config, ConfigResetGuard) {
        let tmp = TempDir::new().expect("tempdir");
        Config::override_root(tmp.path());
        Config::set_database_path(tmp.path().join("memory.db"));
        let cfg = Config::current();
        cfg.ensure_directories().expect("dirs");
        (tmp, cfg, ConfigResetGuard)
    }

    // T-14.1: RTM FR-21
    #[test]
    fn falls_back_to_created_timestamp_when_last_accessed_is_missing() {
        let (_tmp, cfg, _reset) = isolated_config();
        let created = (Utc::now() - Duration::days(30)).to_rfc3339();

        let weight = compute_decay_weight_with_created(
            "memory://notes/created-fallback",
            None,
            Some(&created),
            &cfg,
            None,
        );

        assert!(weight < 1.0, "expected decayed score, got {weight}");
    }

    // T-14.1: RTM FR-21
    #[test]
    fn preserves_recent_last_accessed_behavior() {
        let (_tmp, cfg, _reset) = isolated_config();
        let last_accessed = (Utc::now() - Duration::days(1)).to_rfc3339();

        let weight = compute_decay_weight_with_created(
            "memory://notes/recent-access",
            Some(&last_accessed),
            None,
            &cfg,
            None,
        );

        assert!(weight > 0.95, "expected near-neutral score, got {weight}");
    }

    // T-14.1: RTM FR-21
    #[test]
    fn keeps_graceful_neutral_fallback_when_no_timestamps_exist() {
        let (_tmp, cfg, _reset) = isolated_config();

        let weight =
            compute_decay_weight_with_created("memory://notes/no-dates", None, None, &cfg, None);

        assert_eq!(weight, 1.0);
    }
}

fn grace_period_for_path(file_path: &str, config: &Config) -> i64 {
    let normalized = normalize_decay_path(file_path, config);
    if normalized.contains("/thinking/sequential/")
        || normalized.contains("/sequential/")
        || normalized.contains("/thinking/compaction/")
        || normalized.contains("/compaction/")
    {
        return config.decay_grace_days_sequential();
    }
    if normalized.contains("/thinking/workflow/") || normalized.contains("/thinking/workflows/") {
        return config.decay_grace_days_workflows();
    }
    config.decay_grace_days_default()
}

fn normalize_decay_path(file_path: &str, config: &Config) -> String {
    let normalized = file_path.replace('\\', "/").to_ascii_lowercase();
    let memory_root = config
        .memory_path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();

    if normalized.starts_with("memory://") {
        return normalized.replacen("memory://", &format!("{memory_root}/"), 1);
    }

    normalized
}
