#![cfg(unix)]

use chrono::Utc;
use maenifold_core::markdown::path_to_uri;
use maenifold_core::memory::get_decay_weight_for_file;
use maenifold_core::search::get_file_display_info;
use maenifold_core::{Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::params;
use std::fs;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
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

fn capture_stderr<F, T>(operation: F) -> (T, String)
where
    F: FnOnce() -> T,
{
    unsafe extern "C" {
        fn pipe(fds: *mut i32) -> i32;
        fn dup(fd: i32) -> i32;
        fn dup2(src: i32, dest: i32) -> i32;
        fn close(fd: i32) -> i32;
    }

    let stderr = std::io::stderr();
    let stderr_fd = stderr.as_raw_fd();
    let mut pipe_fds = [0_i32; 2];

    // SAFETY: pipe_fds points to valid writable memory for two file descriptors.
    let pipe_result = unsafe { pipe(pipe_fds.as_mut_ptr()) };
    assert_eq!(pipe_result, 0, "pipe failed");

    // SAFETY: stderr_fd is a valid process file descriptor.
    let saved_stderr = unsafe { dup(stderr_fd) };
    assert!(saved_stderr >= 0, "dup failed");

    // SAFETY: both descriptors are valid and dup2 atomically redirects stderr.
    let redirect_result = unsafe { dup2(pipe_fds[1], stderr_fd) };
    assert!(redirect_result >= 0, "dup2 failed");

    let result = operation();
    std::io::stderr().flush().expect("flush stderr");

    // SAFETY: restore original stderr and close the write end once no more writes are expected.
    assert!(
        unsafe { dup2(saved_stderr, stderr_fd) } >= 0,
        "restore dup2 failed"
    );
    unsafe {
        close(pipe_fds[1]);
        close(saved_stderr);
    }

    let mut output = String::new();
    // SAFETY: pipe_fds[0] is the remaining owned read-end descriptor.
    let mut reader = unsafe { std::fs::File::from_raw_fd(pipe_fds[0]) };
    reader
        .read_to_string(&mut output)
        .expect("read captured stderr");

    (result, output)
}

// T-QUAL-002.6a: RTM FR-25.2
#[test]
fn fr25_2_ac1_decay_fallback_emits_explicit_degraded_signal() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let (weight, stderr) = capture_stderr(|| {
        get_decay_weight_for_file(
            "memory://fr25-2-memory-search-diagnostics/non-existent-file",
            &env.cfg,
        )
    });

    assert_eq!(weight, 1.0);
    assert!(stderr.contains("[DEGRADED][MemorySearchTools.Fusion][GetDecayWeightFromDisk]"));
    assert!(stderr.contains("Falling back to neutral decay weight"));
}

// T-QUAL-002.6a: RTM FR-25.2
#[test]
fn fr25_2_ac1_snippet_fallback_emits_explicit_degraded_signal() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let ((title, snippet), stderr) = capture_stderr(|| {
        get_file_display_info(
            "memory://fr25-2-memory-search-diagnostics/non-existent-file",
            "query",
            &env.cfg,
        )
    });

    assert_eq!(title, "non-existent-file");
    assert!(snippet.starts_with("[DEGRADED] Snippet unavailable"));
    assert!(stderr.contains("[DEGRADED][MemorySearchTools.Fusion][GetFileDisplayInfo]"));
    assert!(stderr.contains("Falling back to filename/snippet"));
}

// T-QUAL-002.6a: RTM FR-25.2
#[test]
fn fr25_2_ac2_success_path_behavior_remains_unchanged() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_note(
        &env,
        "fr25-2-memory-search-diagnostics/valid-note.md",
        "Valid Memory Note",
        "This content contains a targeted query token for snippet extraction.",
    );

    let (((title, snippet), weight), stderr) = capture_stderr(|| {
        (
            get_file_display_info(&uri, "targeted", &env.cfg),
            get_decay_weight_for_file(&uri, &env.cfg),
        )
    });

    assert_eq!(title, "Valid Memory Note");
    assert!(snippet.contains("targeted query token"));
    assert!(!snippet.contains("[DEGRADED]"));
    assert!(weight > 0.0);
    assert!(weight <= 1.0);
    assert!(!stderr.contains("[DEGRADED][MemorySearchTools.Fusion]"));
}

// T-QUAL-002.6a: RTM FR-25.2
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

    assert_eq!(weight, 1.0);
    assert!(
        stderr.contains("[DEGRADED][MemorySearchTools.Fusion][GetDecayWeightForFile(reuse-conn)]")
    );
    assert!(stderr.contains(
        "Malformed file_content.created timestamp; using current UTC as degraded fallback"
    ));
}

// T-QUAL-002.6a: RTM FR-25.2
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
    assert!(snippet.starts_with("[DEGRADED] Snippet unavailable"));
    assert!(
        stderr.contains(&format!("for '{file_name}'")),
        "diagnostics should redact to basename: {stderr}"
    );
    assert!(
        !stderr.contains(&absolute_path.to_string_lossy().to_string()),
        "diagnostics must not leak absolute paths: {stderr}"
    );
}
