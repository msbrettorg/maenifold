// T-QUAL-002.6a: RTM FR-25.2 (archived historical diagnostics requirement, not the
// active FR-25 Azure Deployment Manifests / RTM-137–140). These tests validate the
// degraded-diagnostic contract for the MemorySearchTools.Fusion subsystem: when a
// decay or display-info lookup fails, the tool returns a safe neutral/fallback value
// and emits a structured [DEGRADED][MemorySearchTools.Fusion][Method] line to stderr.
#![cfg(unix)]

use chrono::Utc;
use maenifold_core::markdown::path_to_uri;
use maenifold_core::memory::get_decay_weight_for_file;
use maenifold_core::search::get_file_display_info;
use maenifold_core::{Config, GraphDatabase};
use crate::GLOBAL_CONFIG_TEST_MUTEX as TEST_MUTEX;
use rusqlite::params;
use std::fs;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use tempfile::TempDir;

struct TestEnv {
    _tmp: TempDir,
    cfg: Config,
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

fn isolated_env() -> TestEnv {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    GraphDatabase::initialize_database(&cfg).expect("db init");
    TestEnv { _tmp: tmp, cfg }
}

fn write_memory_note(env: &TestEnv, relative_path: &str, title: &str, body: &str) -> String {
    let note_path = env.cfg.memory_path.join(relative_path);
    if let Some(parent) = note_path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(
        &note_path,
        format!(
            "---\ncreated: {}\ntitle: {}\n---\n\n# {}\n\n{}",
            Utc::now().to_rfc3339(),
            title,
            title,
            body
        ),
    )
    .expect("write note");
    path_to_uri(&note_path, &env.cfg.memory_path).expect("uri")
}

// T-QUAL-002.6a: RTM FR-25.2 — RAII guard for fd restoration on panic.
// Ensures the saved stderr descriptor is restored even if the capture closure panics,
// preventing subsequent tests from writing to a dead pipe.
struct StderrGuard {
    stderr_fd: i32,
    saved_stderr: i32,
    write_end: i32,
}

impl Drop for StderrGuard {
    fn drop(&mut self) {
        unsafe extern "C" {
            fn dup2(src: i32, dest: i32) -> i32;
            fn close(fd: i32) -> i32;
        }
        // SAFETY: restore the saved stderr and close owned write descriptors.
        unsafe {
            dup2(self.saved_stderr, self.stderr_fd);
            close(self.write_end);
            close(self.saved_stderr);
        }
    }
}

fn capture_stderr<F, T>(operation: F) -> (T, String)
where
    F: FnOnce() -> T,
{
    unsafe extern "C" {
        fn pipe(fds: *mut i32) -> i32;
        fn dup(fd: i32) -> i32;
        fn dup2(src: i32, dest: i32) -> i32;
    }

    let stderr = std::io::stderr();
    let stderr_fd = stderr.as_raw_fd();
    let mut pipe_fds = [0_i32; 2];

    // SAFETY: pipe_fds points to writable memory for two descriptors.
    assert_eq!(unsafe { pipe(pipe_fds.as_mut_ptr()) }, 0, "pipe failed");
    // SAFETY: stderr_fd is a valid descriptor.
    let saved_stderr = unsafe { dup(stderr_fd) };
    assert!(saved_stderr >= 0, "dup failed");
    // SAFETY: dup2 atomically redirects the current process stderr descriptor.
    assert!(unsafe { dup2(pipe_fds[1], stderr_fd) } >= 0, "dup2 failed");

    // RAII guard ensures fd restoration even if `operation` panics.
    let guard = StderrGuard {
        stderr_fd,
        saved_stderr,
        write_end: pipe_fds[1],
    };

    let result = operation();
    std::io::stderr().flush().expect("flush stderr");

    // Explicitly drop the guard to restore stderr and close fds before reading.
    drop(guard);

    let mut output = String::new();
    // SAFETY: pipe_fds[0] is the remaining owned read descriptor.
    let mut reader = unsafe { std::fs::File::from_raw_fd(pipe_fds[0]) };
    reader
        .read_to_string(&mut output)
        .expect("read captured stderr");

    (result, output)
}

// T-QUAL-002.6a: RTM FR-25.2
// MERGED from ac1_decay_fallback + ac1_snippet_fallback: both functions degrade
// gracefully when a file does not exist, returning safe fallback values and emitting
// structured [DEGRADED] signals to stderr.
#[test]
fn fr25_2_ac1_fallback_on_missing_file() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let missing_uri = "memory://fr25-2-memory-search-diagnostics/non-existent-file";

    // Exercise both degraded paths in a single capture window.
    let (((title, snippet), weight), stderr) = capture_stderr(|| {
        (
            get_file_display_info(missing_uri, "query", &env.cfg),
            get_decay_weight_for_file(missing_uri, &env.cfg),
        )
    });

    // Decay: neutral fallback weight and structured diagnostic.
    assert_eq!(
        weight, 1.0,
        "missing file should return neutral decay weight"
    );
    assert!(
        stderr.contains("[DEGRADED][MemorySearchTools.Fusion][GetDecayWeightFromDisk]"),
        "decay diagnostic tag missing from stderr: {stderr}"
    );
    assert!(
        stderr.contains("Falling back to neutral decay weight"),
        "decay diagnostic message missing from stderr: {stderr}"
    );

    // Display info: fallback title derived from URI segment, exact degraded snippet.
    assert_eq!(title, "non-existent-file");
    assert_eq!(
        snippet, "[DEGRADED] Snippet unavailable",
        "snippet should be the exact degraded sentinel, got: {snippet}"
    );
    assert!(
        stderr.contains("[DEGRADED][MemorySearchTools.Fusion][GetFileDisplayInfo]"),
        "display diagnostic tag missing from stderr: {stderr}"
    );
    assert!(
        stderr.contains("Falling back to filename/snippet"),
        "display diagnostic message missing from stderr: {stderr}"
    );
}

// T-QUAL-002.6a: RTM FR-25.2
// SPLIT from ac2_success_path: display-info success path (one-concept-per-test).
#[test]
fn fr25_2_ac2a_display_info_success_path_no_degraded_signal() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_note(
        &env,
        "fr25-2-memory-search-diagnostics/valid-note-display.md",
        "Valid Memory Note",
        "This content contains a targeted query token for snippet extraction.",
    );

    let ((title, snippet), stderr) =
        capture_stderr(|| get_file_display_info(&uri, "targeted", &env.cfg));

    assert_eq!(title, "Valid Memory Note");
    assert!(
        snippet.contains("targeted query token"),
        "snippet should contain query match, got: {snippet}"
    );
    assert!(
        !snippet.contains("[DEGRADED]"),
        "success-path snippet must not contain DEGRADED sentinel: {snippet}"
    );
    assert!(
        !stderr.contains("[DEGRADED][MemorySearchTools.Fusion][GetFileDisplayInfo]"),
        "success path must not emit GetFileDisplayInfo degraded signal: {stderr}"
    );
}

// T-QUAL-002.6a: RTM FR-25.2
// SPLIT from ac2_success_path: decay-weight success path (one-concept-per-test).
#[test]
fn fr25_2_ac2b_decay_weight_success_path_no_degraded_signal() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_note(
        &env,
        "fr25-2-memory-search-diagnostics/valid-note-decay.md",
        "Decay Weight Test Note",
        "Content for decay weight validation.",
    );

    let (weight, stderr) = capture_stderr(|| get_decay_weight_for_file(&uri, &env.cfg));

    assert!(
        weight > 0.90,
        "freshly created file should have near-maximal decay weight, got {weight}"
    );
    assert!(
        weight <= 1.0,
        "decay weight must not exceed 1.0, got {weight}"
    );
    assert!(
        !stderr.contains("[DEGRADED][MemorySearchTools.Fusion]"),
        "success path must not emit any MemorySearchTools.Fusion degraded signal: {stderr}"
    );
}

// T-QUAL-002.6a: RTM FR-25.2
// REWRITE: asserts component tag and message content without coupling to the
// internal method sub-label `(reuse-conn)`, which is an implementation detail
// subject to change during the FR-22 DbBackend migration.
#[test]
fn fr25_2_ac3_malformed_created_emits_explicit_degraded_diagnostic() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let malformed_uri = "memory://fr25-2-memory-search-diagnostics/malformed-created-note";

    let connection = rusqlite::Connection::open(&env.cfg.database_path).expect("open db");
    connection
        .execute(
            "INSERT OR REPLACE INTO file_content (file_path, last_indexed, created) VALUES (?1, ?2, ?3)",
            params![malformed_uri, Utc::now().to_rfc3339(), "not-a-valid-iso-timestamp"],
        )
        .expect("insert malformed row");

    let (weight, stderr) = capture_stderr(|| get_decay_weight_for_file(malformed_uri, &env.cfg));

    assert_eq!(
        weight, 1.0,
        "malformed timestamp should still return neutral weight"
    );
    assert!(
        stderr.contains("[DEGRADED][MemorySearchTools.Fusion]"),
        "component tag missing from degraded diagnostic: {stderr}"
    );
    assert!(
        stderr.contains(
            "Malformed file_content.created timestamp; using current UTC as degraded fallback"
        ),
        "malformed-timestamp diagnostic message missing from stderr: {stderr}"
    );
}

// T-QUAL-002.6a: RTM FR-25.2
// KEEP: guards the privacy/security path-redaction property in GetFileDisplayInfo.
#[test]
fn fr25_2_ac4_path_diagnostics_redacts_absolute_paths() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_name = format!("sensitive-note-{}.md", uuid::Uuid::new_v4().simple());
    let absolute_path = std::env::temp_dir()
        .join("fr25-2-sensitive")
        .join(&file_name);

    let ((title, snippet), stderr) = capture_stderr(|| {
        get_file_display_info(&absolute_path.to_string_lossy(), "query", &env.cfg)
    });

    assert_eq!(
        title,
        std::path::Path::new(&file_name)
            .file_stem()
            .expect("file stem")
            .to_string_lossy()
    );
    assert_eq!(
        snippet, "[DEGRADED] Snippet unavailable",
        "snippet should be the exact degraded sentinel, got: {snippet}"
    );
    assert!(
        stderr.contains(&format!("for '{file_name}'")),
        "diagnostics should redact to basename: {stderr}"
    );
    assert!(
        !stderr.contains(&absolute_path.to_string_lossy().to_string()),
        "diagnostics must not leak absolute paths: {stderr}"
    );
}

// T-QUAL-002.6a: RTM FR-25.2
// NEW: companion to ac4 covering the parallel path-redaction code path in
// decay/diagnostics.rs::redact_diagnostic_path via get_decay_weight_for_file.
#[test]
fn fr25_2_ac4b_decay_diagnostics_redacts_absolute_paths() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_name = format!("sensitive-decay-{}.md", uuid::Uuid::new_v4().simple());
    let absolute_path = std::env::temp_dir()
        .join("fr25-2-sensitive-decay")
        .join(&file_name);

    let (weight, stderr) =
        capture_stderr(|| get_decay_weight_for_file(&absolute_path.to_string_lossy(), &env.cfg));

    assert_eq!(
        weight, 1.0,
        "non-existent absolute path should return neutral decay weight"
    );
    assert!(
        stderr.contains("[DEGRADED][MemorySearchTools.Fusion][GetDecayWeightFromDisk]"),
        "decay diagnostic tag missing from stderr: {stderr}"
    );
    assert!(
        stderr.contains(&format!("for '{file_name}'")),
        "decay diagnostics should redact to basename: {stderr}"
    );
    assert!(
        !stderr.contains(&absolute_path.to_string_lossy().to_string()),
        "decay diagnostics must not leak absolute paths: {stderr}"
    );
}
