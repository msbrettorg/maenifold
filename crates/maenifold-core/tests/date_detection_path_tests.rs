// T-DATE-001: RTM-001, RTM-002 — Ported DateDetectionPathTests.cs coverage for thinking session path dates.

use maenifold_core::Config;
use maenifold_core::paths::{dated_sequential_path, dated_thinking_path};
use once_cell::sync::Lazy;
use parking_lot::{Mutex, MutexGuard};
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

struct TestEnv {
    _guard: MutexGuard<'static, ()>,
    _tmp: TempDir,
    config: Config,
}

impl TestEnv {
    fn new() -> Self {
        let guard = TEST_MUTEX.lock();
        let tmp = TempDir::new().expect("tmp");
        Config::override_root(tmp.path());
        Config::set_database_path(tmp.path().join("memory.db"));
        let config = Config::current();
        config.ensure_directories().expect("dirs");
        Self {
            _guard: guard,
            _tmp: tmp,
            config,
        }
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

fn path_fragment(segments: &[&str]) -> String {
    let mut path = std::path::PathBuf::new();
    for segment in segments {
        path.push(segment);
    }
    path.to_string_lossy().into_owned()
}

// REWRITE: Dropped redundant `contains` assertion (subsumed by `ends_with`).
// Consolidated epoch-regression concern from removed Test 3 as comment.
#[test]
fn get_session_path_sequential_session_id_parses_timestamp_to_correct_date() {
    let env = TestEnv::new();

    // 1770910549160 ms = 2026-02-12 UTC
    let path = dated_sequential_path("session-1770910549160-79869", &env.config);
    let rendered = path.to_string_lossy();

    // T-DATE-001: RTM-001, RTM-002 — Must parse the millisecond timestamp from segment[1]
    // after splitting on '-'. Regression: the old bug parsed "session" (segment[0]) as the
    // timestamp, yielding epoch 0 → 1970/01/01. Correct date is 2026-02-12.
    assert!(rendered.ends_with(&path_fragment(&[
        "thinking",
        "sequential",
        "2026",
        "02",
        "12",
        "session-1770910549160-79869.md",
    ])));
}

// REWRITE+MERGE: Dropped redundant `contains` assertion. Merged Test 6's negative assertion
// to guard that session_type is not hardcoded to "sequential".
#[test]
fn get_session_path_workflow_session_id_parses_timestamp_to_correct_date() {
    let env = TestEnv::new();

    // 1770910549160 ms = 2026-02-12 UTC
    let path = dated_thinking_path("workflow", "workflow-1770910549160", &env.config);
    let rendered = path.to_string_lossy();

    // T-DATE-001: RTM-001 — Must handle 2-segment session IDs without a trailing random suffix.
    assert!(rendered.ends_with(&path_fragment(&[
        "thinking",
        "workflow",
        "2026",
        "02",
        "12",
        "workflow-1770910549160.md",
    ])));
    // T-DATE-001: RTM-002 — session_type must not be hardcoded to "sequential".
    assert!(!rendered.contains(&path_fragment(&["thinking", "sequential"])));
}

// KEEP: Only multi-case test with distinct timestamps (2021, 2023). Genuinely load-bearing.
#[test]
fn get_session_path_different_timestamps_produce_different_date_directories() {
    let env = TestEnv::new();

    // 1609459200000 ms = 2021-01-01 00:00:00 UTC
    let path_a = dated_sequential_path("session-1609459200000-12345", &env.config);
    let rendered_a = path_a.to_string_lossy();

    // 1672531200000 ms = 2023-01-01 00:00:00 UTC
    let path_b = dated_sequential_path("session-1672531200000-12345", &env.config);
    let rendered_b = path_b.to_string_lossy();

    // T-DATE-001: RTM-001 — The parsed timestamps must drive the date directory.
    assert!(rendered_a.contains(&path_fragment(&["2021", "01", "01"])));
    assert!(rendered_b.contains(&path_fragment(&["2023", "01", "01"])));
    assert_ne!(path_a, path_b);
}

// NEW (Recommendation 4, P1): Exercises the Utc::now() fallback path in date_parts_from_session_id
// when segment[1] is not a valid numeric timestamp. This was a complete coverage gap.
#[test]
fn get_session_path_with_non_numeric_segment_falls_back_to_today_not_epoch() {
    let env = TestEnv::new();

    // "session-abc-123" → segment[1] = "abc" → parse::<i64> fails → fallback to Utc::now()
    let path = dated_sequential_path("session-abc-123", &env.config);
    let rendered = path.to_string_lossy();

    // T-DATE-001: RTM-002 — Fallback must not produce 1970 (epoch). It should produce
    // today's date via Utc::now(). We assert the absence of epoch directories rather than
    // an exact date since the value is runtime-dependent.
    assert!(!rendered.contains(&path_fragment(&["thinking", "sequential", "1970"])));
    // Must still produce a valid directory structure with the session ID as the filename.
    assert!(rendered.ends_with("session-abc-123.md"));
    assert!(rendered.contains(&path_fragment(&["thinking", "sequential"])));
}
