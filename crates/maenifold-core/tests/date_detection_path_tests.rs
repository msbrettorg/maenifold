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

#[test]
fn get_session_path_sequential_session_id_parses_timestamp_to_correct_date() {
    let env = TestEnv::new();

    // 1770910549160 ms = 2026-02-12 UTC
    let path = dated_sequential_path("session-1770910549160-79869", &env.config);
    let rendered = path.to_string_lossy();

    // T-DATE-001: RTM-001 — Must parse the timestamp after stripping the `session-` prefix.
    assert!(rendered.contains(&path_fragment(&["2026", "02", "12"])));
    assert!(rendered.ends_with(&path_fragment(&[
        "thinking",
        "sequential",
        "2026",
        "02",
        "12",
        "session-1770910549160-79869.md",
    ])));
}

#[test]
fn get_session_path_workflow_session_id_parses_timestamp_to_correct_date() {
    let env = TestEnv::new();

    // 1770910549160 ms = 2026-02-12 UTC
    let path = dated_thinking_path("workflow", "workflow-1770910549160", &env.config);
    let rendered = path.to_string_lossy();

    // T-DATE-001: RTM-001 — Must handle session IDs without a trailing random suffix.
    assert!(rendered.contains(&path_fragment(&["2026", "02", "12"])));
    assert!(rendered.ends_with(&path_fragment(&[
        "thinking",
        "workflow",
        "2026",
        "02",
        "12",
        "workflow-1770910549160.md",
    ])));
}

#[test]
fn get_session_path_modern_timestamp_does_not_produce_epoch_date() {
    let env = TestEnv::new();

    let path = dated_sequential_path("session-1770910549160-79869", &env.config);
    let rendered = path.to_string_lossy();

    // T-DATE-001: RTM-002 — The old bug parsed `session` as the timestamp, yielding epoch 0 -> 1970/01/01.
    assert!(!rendered.contains("1970"));
}

#[test]
fn get_session_path_returns_correct_directory_structure() {
    let env = TestEnv::new();

    let path = dated_sequential_path("session-1770910549160-79869", &env.config);
    let rendered = path.to_string_lossy();

    // T-DATE-001: RTM-002 — Full expected path suffix.
    assert!(rendered.ends_with(&path_fragment(&[
        "thinking",
        "sequential",
        "2026",
        "02",
        "12",
        "session-1770910549160-79869.md",
    ])));
}

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

#[test]
fn get_session_path_thinking_type_propagates_correctly() {
    let env = TestEnv::new();

    let path = dated_thinking_path("workflow", "workflow-1770910549160", &env.config);
    let rendered = path.to_string_lossy();

    // T-DATE-001: RTM-002 — thinkingType must appear in the path under /thinking/.
    assert!(rendered.contains(&path_fragment(&["thinking", "workflow"])));
    assert!(!rendered.contains(&path_fragment(&["thinking", "sequential"])));
}
