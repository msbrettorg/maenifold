use chrono::{Duration, Utc};
use maenifold_core::{Config, memory, paths};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn isolated_config() -> (TempDir, Config) {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    (tmp, cfg)
}

#[test]
fn memory_uri_to_path_resolves_nested_uri_under_memory_root() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, cfg) = isolated_config();

    let path = paths::memory_uri_to_path("memory://nested/deep/read-note", &cfg).expect("uri path");

    assert_eq!(
        path,
        cfg.memory_path
            .join("nested")
            .join("deep")
            .join("read-note.md")
    );
    Config::reset_overrides();
}

// T-15.19: RTM FR-12 — URI already ending in .md must not get double extension
#[test]
fn memory_uri_to_path_does_not_double_extend_md_uri() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, cfg) = isolated_config();

    let path = paths::memory_uri_to_path("memory://note.md", &cfg).expect("uri path");

    assert_eq!(
        path.file_name().and_then(|s| s.to_str()),
        Some("note.md"),
        "URI already ending in .md should not become note.md.md"
    );
    assert_eq!(path.extension().and_then(|s| s.to_str()), Some("md"));
    Config::reset_overrides();
}

#[test]
fn validate_relative_path_rejects_absolute_paths() {
    let absolute = if cfg!(windows) {
        r"C:\escape"
    } else {
        "/escape"
    };

    let err =
        paths::validate_relative_path(absolute).expect_err("absolute path should be rejected");

    assert!(
        err.to_string().contains("Directory traversal")
            || err.to_string().contains("Absolute")
            || err.to_string().contains("Path traversal")
    );
}

#[test]
fn validate_relative_path_rejects_dot_sequence_traversal() {
    for candidate in [
        "../escape",
        "safe/../escape",
        ".. ",
        ". .",
        "safe/. ./escape",
    ] {
        let err = paths::validate_relative_path(candidate)
            .expect_err("dot-sequence traversal should be rejected");
        assert!(
            err.to_string().contains("Path traversal"),
            "unexpected error for {candidate}: {err}"
        );
    }
}

#[test]
fn validate_relative_path_accepts_valid_nested_relative_folder() {
    let path = paths::validate_relative_path("memory-tests/nested").expect("valid relative path");

    assert_eq!(
        path.to_string_lossy().replace('\\', "/"),
        "memory-tests/nested"
    );
}

// T-14.5: RTM FR-12 — slugify lowercases and hyphenates spaces
#[test]
fn slugify_title_lowercases_and_hyphenates_spaces() {
    assert_eq!(
        paths::slugify_title("Valid Relative Path"),
        "valid-relative-path"
    );
}

#[test]
fn validate_relative_path_rejects_url_encoded_traversal_sequences() {
    for candidate in ["%2e%2e/escape", "safe/%2E%2E/escape", "%2e%2e%2fescape"] {
        let err = paths::validate_relative_path(candidate)
            .expect_err("encoded traversal should be rejected");
        assert!(
            err.to_string().contains("Path traversal components"),
            "unexpected error for {candidate}: {err}"
        );
    }
}

#[test]
fn memory_uri_to_path_rejects_traversal_uri() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, cfg) = isolated_config();

    let err = paths::memory_uri_to_path("memory://safe/../escape", &cfg)
        .expect_err("traversal uri should be rejected");

    assert!(err.to_string().contains("Path traversal"));
    Config::reset_overrides();
}

#[test]
fn slugify_title_falls_back_to_untitled_when_all_characters_are_filtered() {
    assert_eq!(paths::slugify_title("!!! ### ***"), "untitled");
}

#[test]
fn sanitize_user_input_strips_url_encoded_path_traversal_sequences() {
    assert_eq!(
        memory::sanitize_user_input("test%2e%2e%2fmalicious"),
        "testmalicious"
    );
}

#[test]
fn sanitize_user_input_strips_mixed_encoded_and_plain_path_traversal_sequences() {
    assert_eq!(memory::sanitize_user_input("%2e./test"), "test");
}

#[test]
fn compute_decay_weight_defaults_to_one_without_last_accessed() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, cfg) = isolated_config();

    let weight = memory::compute_decay_weight("memory://notes/example", None, &cfg, None);

    assert_eq!(weight, 1.0);
    Config::reset_overrides();
}

#[test]
fn compute_decay_weight_uses_shorter_sequential_grace_period_than_default() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, cfg) = isolated_config();

    // T-12.5: RTM FR-21 — guard: test relies on default config values
    assert_eq!(
        cfg.decay_grace_days_sequential(),
        7,
        "test assumes default sequential grace of 7 days"
    );
    assert_eq!(
        cfg.decay_grace_days_default(),
        28,
        "test assumes default grace of 28 days"
    );

    let last_accessed = (Utc::now() - Duration::days(10)).to_rfc3339();

    let sequential = memory::compute_decay_weight(
        "memory://thinking/sequential/2026/03/session-note",
        Some(&last_accessed),
        &cfg,
        None,
    );
    let default = memory::compute_decay_weight(
        "memory://notes/reference-note",
        Some(&last_accessed),
        &cfg,
        None,
    );

    assert!(
        sequential < default,
        "sequential memories should decay faster after grace period: sequential={sequential}, default={default}"
    );
    Config::reset_overrides();
}

#[test]
fn compute_decay_weight_keeps_validated_assumptions_at_one() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, cfg) = isolated_config();
    let last_accessed = (Utc::now() - Duration::days(90)).to_rfc3339();

    // T-7.8: RTM FR-7.8
    let weight = memory::compute_decay_weight(
        "memory://assumptions/2026/03/validated-note",
        Some(&last_accessed),
        &cfg,
        Some("validated"),
    );

    assert_eq!(weight, 1.0);
    Config::reset_overrides();
}

#[test]
fn compute_decay_weight_decays_invalidated_assumptions_faster_than_active() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, cfg) = isolated_config();

    // T-7.8: RTM FR-7.8
    // Note: assumption status grace/half-life values are hardcoded in scoring.rs
    // (invalidated: grace=7d, hl=14d; active: grace=14d, hl=30d), so config env vars
    // do not affect this test. The 20-day age ensures both statuses have exited their
    // grace periods: invalidated decayed for 13d (weight ≈ 0.53), active for 6d (weight ≈ 0.87).
    let last_accessed = (Utc::now() - Duration::days(20)).to_rfc3339();

    let invalidated = memory::compute_decay_weight(
        "memory://assumptions/2026/03/invalidated-note",
        Some(&last_accessed),
        &cfg,
        Some("invalidated"),
    );
    let active = memory::compute_decay_weight(
        "memory://assumptions/2026/03/active-note",
        Some(&last_accessed),
        &cfg,
        Some("active"),
    );

    assert!(
        invalidated < active,
        "invalidated assumptions should decay faster: invalidated={invalidated}, active={active}"
    );
    Config::reset_overrides();
}

// T-14.5: RTM FR-12 — sanitize_user_input must be a no-op on safe input
#[test]
fn sanitize_user_input_passes_clean_input_unchanged() {
    assert_eq!(memory::sanitize_user_input("hello world"), "hello world");
}
