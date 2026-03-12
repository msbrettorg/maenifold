use serde_json::Value;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
struct CliRun {
    exit_code: i32,
    stdout: String,
    stderr: String,
}

impl CliRun {
    fn success(&self) -> bool {
        self.exit_code == 0
    }
}

#[derive(Debug)]
struct IsolatedRoot {
    root_dir: PathBuf,
    database_path: PathBuf,
}

impl IsolatedRoot {
    fn new(label: &str) -> io::Result<Self> {
        let nonce = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let safe_label = label
            .chars()
            .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
            .collect::<String>();
        let root_dir = env::temp_dir().join("maenifold-rs-workspace").join(format!(
            "{safe_label}-{timestamp}-{nonce}-{}",
            std::process::id()
        ));
        let database_path = root_dir.join("memory.db");

        fs::create_dir_all(root_dir.join("memory"))?;
        fs::create_dir_all(root_dir.join("assets"))?;

        Ok(Self {
            root_dir,
            database_path,
        })
    }

    fn apply_to_command<'a>(&self, command: &'a mut Command) -> &'a mut Command {
        command
            .env("MAENIFOLD_ROOT", &self.root_dir)
            .env("MAENIFOLD_DATABASE_PATH", &self.database_path)
    }
}

impl Drop for IsolatedRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root_dir);
    }
}

fn run_rust_cli(args: &[&str], sandbox: &IsolatedRoot) -> io::Result<CliRun> {
    if let Some(program) = env::var_os("MAENIFOLD_RUST_CLI")
        .or_else(|| env::var_os("CARGO_BIN_EXE_maenifold-cli"))
        .or_else(|| env::var_os("CARGO_BIN_EXE_maenifold"))
        .map(PathBuf::from)
    {
        let mut command = Command::new(program);
        command.args(args);
        sandbox.apply_to_command(&mut command);
        return capture(command);
    }

    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut command = Command::new("cargo");
    command
        .current_dir(workspace_root)
        .arg("run")
        .arg("-q")
        .arg("-p")
        .arg("maenifold-cli")
        .arg("--");
    command.args(args);
    sandbox.apply_to_command(&mut command);
    capture(command)
}

fn capture(mut command: Command) -> io::Result<CliRun> {
    let output = command.output()?;
    Ok(CliRun {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    })
}

fn parse_envelope(stdout: &str) -> Value {
    serde_json::from_str(stdout).expect("valid json envelope")
}

#[test]
fn workspace_cli_smoke_is_rust_only() {
    let sandbox = IsolatedRoot::new("workspace-cli-smoke").expect("sandbox");

    let write = run_rust_cli(
        &[
            "--tool",
            "WriteMemory",
            "--payload",
            r#"{"title":"Workspace Smoke","content":"Testing [[workspace-smoke]] with [[rust-only-validation]]."}"#,
            "--json",
        ],
        &sandbox,
    )
    .expect("write command");
    assert!(write.success(), "write failed: {}", write.stderr);
    let write_json = parse_envelope(&write.stdout);
    assert_eq!(write_json["success"], Value::Bool(true));
    let uri = write_json["data"]["uri"]
        .as_str()
        .expect("write uri")
        .to_string();

    let sync = run_rust_cli(
        &["--tool", "Sync", "--payload", "{}", "--json"],
        &sandbox,
    )
    .expect("sync command");
    assert!(sync.success(), "sync failed: {}", sync.stderr);
    let sync_json = parse_envelope(&sync.stdout);
    assert_eq!(sync_json["success"], Value::Bool(true));

    let read_payload = format!(r#"{{"identifier":"{uri}"}}"#);
    let read = run_rust_cli(
        &["--tool", "ReadMemory", "--payload", &read_payload, "--json"],
        &sandbox,
    )
    .expect("read command");
    assert!(read.success(), "read failed: {}", read.stderr);
    let read_json = parse_envelope(&read.stdout);
    assert_eq!(read_json["success"], Value::Bool(true));
    assert_eq!(
        read_json["data"]["title"].as_str(),
        Some("Workspace Smoke")
    );

    let search = run_rust_cli(
        &[
            "--tool",
            "SearchMemories",
            "--payload",
            r#"{"query":"workspace-smoke","mode":"Hybrid"}"#,
            "--json",
        ],
        &sandbox,
    )
    .expect("search command");
    assert!(search.success(), "search failed: {}", search.stderr);
    let search_json = parse_envelope(&search.stdout);
    assert_eq!(search_json["success"], Value::Bool(true));
    assert!(
        search_json["data"]["totalCount"]
            .as_u64()
            .expect("search total count")
            >= 1
    );

    let context = run_rust_cli(
        &[
            "--tool",
            "BuildContext",
            "--payload",
            r#"{"conceptName":"workspace-smoke","depth":1,"maxEntities":10}"#,
            "--json",
        ],
        &sandbox,
    )
    .expect("build context command");
    assert!(context.success(), "build context failed: {}", context.stderr);
    let context_json = parse_envelope(&context.stdout);
    assert_eq!(context_json["success"], Value::Bool(true));
}
