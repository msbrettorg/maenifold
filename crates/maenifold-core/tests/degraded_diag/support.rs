use maenifold_core::json::OutputFormat;
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::fs;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::path::PathBuf;
use tempfile::TempDir;

// T-12.7: RTM NFR-1
pub static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub struct TestEnv {
    pub _tmp: TempDir,
    pub cfg: Config,
}

impl TestEnv {
    pub fn setup() -> Self {
        let tmp = TempDir::new().expect("tmp");
        Config::override_root(tmp.path());
        Config::set_database_path(tmp.path().join("memory.db"));
        let cfg = Config::current();
        cfg.ensure_directories().expect("dirs");
        GraphDatabase::initialize_database(&cfg).expect("db init");
        Self { _tmp: tmp, cfg }
    }

    pub fn markdown_context(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Markdown)
    }

    pub fn json_context(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Json)
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

pub fn capture_stderr<F, T>(operation: F) -> (T, String)
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

    // SAFETY: pipe_fds points to writable memory for two descriptors.
    assert_eq!(unsafe { pipe(pipe_fds.as_mut_ptr()) }, 0, "pipe failed");
    // SAFETY: stderr_fd is a valid descriptor.
    let saved_stderr = unsafe { dup(stderr_fd) };
    assert!(saved_stderr >= 0, "dup failed");
    // SAFETY: dup2 atomically redirects the current process stderr descriptor.
    assert!(unsafe { dup2(pipe_fds[1], stderr_fd) } >= 0, "dup2 failed");

    let result = operation();
    std::io::stderr().flush().expect("flush stderr");

    // SAFETY: restore the saved stderr and close owned write descriptors.
    assert!(
        unsafe { dup2(saved_stderr, stderr_fd) } >= 0,
        "restore dup2 failed"
    );
    unsafe {
        close(pipe_fds[1]);
        close(saved_stderr);
    }

    let mut output = String::new();
    // SAFETY: pipe_fds[0] is the remaining owned read descriptor.
    let mut reader = unsafe { std::fs::File::from_raw_fd(pipe_fds[0]) };
    reader
        .read_to_string(&mut output)
        .expect("read captured stderr");

    (result, output)
}

pub fn write_memory_file(env: &TestEnv, relative_path: &str, content: &str) -> PathBuf {
    let path = env.cfg.memory_path.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(&path, content).expect("write memory file");
    path
}

pub fn open_connection(env: &TestEnv) -> rusqlite::Connection {
    let conn = rusqlite::Connection::open(&env.cfg.database_path).expect("open db");
    conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;")
        .expect("pragmas");
    conn
}
