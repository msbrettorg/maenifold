use super::{
    LockGuard, RunRecord, RunStatus, ScheduleStore, ScheduledTask,
    env_allowlist::{default_allowlist, matches},
};
use anyhow::{Context, Result};
use chrono::Utc;
#[path = "supervisor_mcp.rs"]
mod supervisor_mcp;
use serde_json::json;
use std::fs::{self, OpenOptions};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use supervisor_mcp::{ExecutionOutcome, build_invocation};
#[cfg(test)]
use supervisor_mcp::parse_mcp_action;
use tracing::info;

#[cfg(test)]
#[path = "supervisor_tests.rs"]
mod supervisor_tests;

pub struct Supervisor {
    store: ScheduleStore,
    maenifold_bin: PathBuf,
    lock_dir: PathBuf,
}

impl Supervisor {
    pub fn new(store: ScheduleStore, maenifold_bin: PathBuf) -> Self {
        let maenifold_bin = if maenifold_bin.is_absolute() {
            maenifold_bin
        } else {
            match std::env::current_dir() {
                Ok(current_dir) => current_dir.join(maenifold_bin),
                Err(_) => maenifold_bin,
            }
        };
        Self {
            lock_dir: store.base_dir().join("locks"),
            store,
            maenifold_bin,
        }
    }
    /// Execute a scheduled task with lock, timeout, and logging
    pub fn run(&self, task_id: &str) -> Result<RunRecord> {
        let mut task = self.store.read(task_id)?;
        let started_at = Utc::now();
        let started = Instant::now();
        let _guard = match LockGuard::try_acquire(&self.lock_dir, task_id) {
            Ok(lock) => Some(lock),
            Err(error) if is_already_running(&error) => {
                let message = error.to_string();
                info!(task_id, error = %message, "skipping overlapping scheduled task");
                let record = self.record(&mut task, RunStatus::Skipped, Some(message), None, None, None, None, started_at, started)?;
                return Ok(record);
            }
            Err(error) => return Err(error),
        };

        let mcp_action = task.mcp_action.clone();
        let mcp_parameters = mcp_action.as_ref().map(|_| task.mcp_parameters.clone().unwrap_or_else(|| json!({})));
        let outcome = self.execute_with_timeout(&task);
        let (status, error, mcp_result, sidecar_path) = match outcome {
            Ok(result) => {
                let mcp_result = mcp_action.as_ref().and_then(|_| serde_json::from_str::<serde_json::Value>(&result.stdout).ok());
                (result.status, result.error, mcp_result, Some(result.sidecar_path))
            }
            Err(error) => (RunStatus::Failed, Some(error.to_string()), None, None),
        };
        self.record(&mut task, status, error, mcp_action, mcp_parameters, mcp_result, sidecar_path, started_at, started)
    }

    fn record(
        &self,
        task: &mut ScheduledTask,
        status: RunStatus,
        error: Option<String>,
        mcp_action: Option<String>,
        mcp_parameters: Option<serde_json::Value>,
        mcp_result: Option<serde_json::Value>,
        sidecar_path: Option<PathBuf>,
        timestamp: chrono::DateTime<Utc>,
        started: Instant,
    ) -> Result<RunRecord> {
        let record = RunRecord {
            status,
            timestamp,
            duration_seconds: started.elapsed().as_secs(),
            error,
            mcp_action,
            mcp_parameters,
            mcp_result,
        };
        task.last_run = Some(record.clone());
        self.store.update(task)?;
        let sidecar_path = sidecar_path.unwrap_or_else(|| {
            self.store.task_dir(&task.id).join("logs").join(format!("{}.json", timestamp.format("%Y%m%dT%H%M%SZ")))
        });
        supervisor_mcp::write_runrecord_sidecar(&record, &sidecar_path);
        Ok(record)
    }

    fn execute_with_timeout(&self, task: &ScheduledTask) -> Result<ExecutionOutcome> {
        let log_path = self.log_path(&task.id);
        let log_dir = log_path
            .parent()
            .context("scheduled task log path has no parent directory")?;
        fs::create_dir_all(log_dir)
            .with_context(|| format!("failed to create log directory '{}'", log_path.display()))?;
        let mut log_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&log_path)
            .with_context(|| format!("failed to create log file '{}'", log_path.display()))?;
        let workdir = match task.workdir.as_deref() {
            None => None,
            Some(dir) if dir.is_absolute() => Some(dir.to_path_buf()),
            Some(dir) => Some(
                std::env::current_dir()
                    .context("failed to resolve current directory for scheduled task workdir")?
                    .join(dir),
            ),
        };
        let invocation = build_invocation(task, workdir.as_deref())?;
        let json_suffix = if invocation.args.iter().any(|arg| arg == "--json") {
            " --json"
        } else {
            ""
        };
        writeln!(
            log_file,
            "started_at={} command={} tool {} {}{}",
            Utc::now().to_rfc3339(),
            self.maenifold_bin.display(),
            invocation.tool_name,
            invocation.payload,
            json_suffix
        )
        .with_context(|| format!("failed to write log header '{}'", log_path.display()))?;
        log_file
            .sync_all()
            .with_context(|| format!("failed to sync log file '{}'", log_path.display()))?;
        let stderr = Stdio::from(
            log_file
                .try_clone()
                .with_context(|| format!("failed to clone log file '{}'", log_path.display()))?,
        );
        let mut command = Command::new(&self.maenifold_bin);
        command.args(&invocation.args).stdout(Stdio::piped()).stderr(stderr);
        if let Some(dir) = workdir.as_deref() {
            command.current_dir(dir);
        }
        apply_env_scrub(&mut command);
        configure_command(&mut command);
        let mut child = command.spawn().with_context(|| {
            format!(
                "failed to spawn maenifold for scheduled task '{}' using '{}'",
                task.id,
                self.maenifold_bin.display()
            )
        })?;
        let stdout_reader = supervisor_mcp::capture_stdout(&mut child, log_file.try_clone()?);
        let timeout = Duration::from_secs(u64::from(task.timeout_seconds));
        let started = Instant::now();
        let (status, error) = loop {
            match child
                .try_wait()
                .context("failed to poll scheduled task child")?
            {
                Some(status) if status.success() => break (RunStatus::Success, None),
                Some(status) => {
                    let detail = match status.code() {
                        Some(code) => format!("exit code: {code}"),
                        None => "process terminated by signal".to_string(),
                    };
                    break (
                        RunStatus::Failed,
                        Some(format!("{detail} (log: {})", log_path.display())),
                    );
                }
                None if started.elapsed() > timeout => {
                    terminate_child(&mut child)?;
                    break (
                        RunStatus::Timeout,
                        Some(format!(
                            "timed out after {} seconds (log: {})",
                            task.timeout_seconds,
                            log_path.display()
                        )),
                    );
                }
                None => thread::sleep(Duration::from_millis(500)),
            }
        };
        let stdout = supervisor_mcp::join_stdout(stdout_reader)?;
        Ok(supervisor_mcp::ExecutionOutcome { status, error, stdout, sidecar_path: log_path.with_extension("json") })
    }

    fn log_path(&self, task_id: &str) -> PathBuf {
        let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        self.store
            .task_dir(task_id)
            .join("logs")
            .join(format!("{timestamp}.log"))
    }
}

/// T-305.4: scheduled fire-time children must not inherit the daemon's
/// full launchd/user environment. Re-apply only the explicit allowlist so
/// API keys, tokens, and SSH agent sockets are stripped before scheduled tools.
fn apply_env_scrub(command: &mut Command) {
    let allowlist = default_allowlist();
    command.env_clear();
    for (key, value) in std::env::vars() {
        if matches(&allowlist, &key) {
            command.env(key, value);
        }
    }
}

fn build_payload(task: &ScheduledTask, workdir: Option<&Path>) -> Result<String> {
    let mut payload = json!({
        "agent": &task.agent,
        "prompt": &task.prompt,
        "mode": "prompt",
        "autoGrantPermissions": task.auto_grant_permissions,
        "timeoutSeconds": task.timeout_seconds,
    });
    if !task.allowed_tools.is_empty() {
        payload["allowedTools"] = json!(&task.allowed_tools);
    }
    if let Some(dir) = workdir {
        payload["cwd"] = json!(dir.display().to_string());
    }
    serde_json::to_string(&payload).context("failed to serialize AcpExec payload")
}
fn is_already_running(error: &anyhow::Error) -> bool {
    error.to_string().contains("already running (PID ")
}

#[cfg(unix)]
fn configure_command(command: &mut Command) {
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(())
            }
        });
    }
}
#[cfg(not(unix))]
fn configure_command(_command: &mut Command) {}

fn terminate_child(child: &mut Child) -> Result<()> {
    #[cfg(unix)]
    if let Ok(pid) = i32::try_from(child.id()) {
        signal_process_group(pid, libc::SIGTERM)?;
        if wait_for_exit(child, Duration::from_secs(5))? {
            return Ok(());
        }
        signal_process_group(pid, libc::SIGKILL)?;
        let _ = wait_for_exit(child, Duration::from_secs(1))?;
        return Ok(());
    }

    child
        .kill()
        .context("failed to kill timed out scheduled task child")?;
    let _ = child.wait();
    Ok(())
}
fn wait_for_exit(child: &mut Child, timeout: Duration) -> Result<bool> {
    let started = Instant::now();
    while started.elapsed() <= timeout {
        if child.try_wait()?.is_some() {
            return Ok(true);
        }
        thread::sleep(Duration::from_millis(100));
    }
    Ok(false)
}
#[cfg(unix)]
fn signal_process_group(pid: i32, signal: i32) -> Result<()> {
    let result = unsafe { libc::killpg(pid, signal) };
    if result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
        return Ok(());
    }
    Err(std::io::Error::last_os_error()).with_context(|| {
        format!("failed to send signal {signal} to scheduled task process group {pid}")
    })
}
