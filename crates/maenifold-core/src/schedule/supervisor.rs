use super::{RunRecord, RunStatus, ScheduleStore, ScheduledTask};
use anyhow::{Context, Result, bail};
use chrono::Utc;
use serde_json::json;
use std::fs::{self, OpenOptions};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
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
            lock_dir: store.base_dir().to_path_buf(),
            store,
            maenifold_bin,
        }
    }
    /// Execute a scheduled task with lock, timeout, and logging
    pub fn run(&self, task_id: &str) -> Result<RunRecord> {
        let mut task = self.store.read(task_id)?;
        let started_at = Utc::now();
        let started = Instant::now();
        let _lock = match self.acquire_lock(task_id) {
            Ok(lock) => Some(lock),
            Err(error) if is_already_running(&error) => {
                let status = if task.skip_if_running {
                    RunStatus::Skipped
                } else {
                    RunStatus::Failed
                };
                let record = self.record(&mut task, status, Some(error.to_string()), started_at, started)?;
                return Ok(record);
            }
            Err(error) => return Err(error),
        };

        let outcome = self.execute_with_timeout(&task);
        let (status, error) = match outcome {
            Ok(result) => result,
            Err(error) => (RunStatus::Failed, Some(error.to_string())),
        };
        self.record(&mut task, status, error, started_at, started)
    }
    fn record(&self, task: &mut ScheduledTask, status: RunStatus, error: Option<String>, timestamp: chrono::DateTime<Utc>, started: Instant) -> Result<RunRecord> {
        let record = RunRecord {
            status,
            timestamp,
            duration_seconds: started.elapsed().as_secs(),
            error,
        };
        task.last_run = Some(record.clone());
        self.store.update(task)?;
        Ok(record)
    }
    fn acquire_lock(&self, task_id: &str) -> Result<LockGuard> {
        let lock_path = self.lock_path(task_id);
        fs::create_dir_all(lock_path.parent().unwrap_or_else(|| Path::new(".")))
            .with_context(|| format!("failed to create lock directory '{}'", lock_path.display()))?;
        for _ in 0..8 {
            match OpenOptions::new().write(true).create_new(true).open(&lock_path) {
                Ok(mut file) => {
                    write!(file, "{}", std::process::id())
                        .with_context(|| format!("failed to write lock '{}'", lock_path.display()))?;
                    file.sync_all().with_context(|| {
                        format!("failed to sync lock file '{}'", lock_path.display())
                    })?;
                    return Ok(LockGuard { path: lock_path });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    if let Some(pid) = read_lock_pid(&lock_path)? {
                        if pid_is_alive(pid) {
                            bail!("scheduled task '{}' is already running (pid {})", task_id, pid);
                        }
                    }
                    match fs::remove_file(&lock_path) {
                        Ok(()) => {}
                        Err(remove_error)
                            if remove_error.kind() == std::io::ErrorKind::NotFound => {}
                        Err(remove_error) => {
                            return Err(remove_error).with_context(|| {
                                format!("failed to remove stale lock '{}'", lock_path.display())
                            });
                        }
                    }
                }
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("failed to create lock '{}'", lock_path.display()));
                }
            }
        }
        bail!("failed to acquire lock '{}'", lock_path.display())
    }
    fn execute_with_timeout(&self, task: &ScheduledTask) -> Result<(RunStatus, Option<String>)> {
        let log_path = self.log_path(&task.id);
        fs::create_dir_all(log_path.parent().unwrap_or_else(|| Path::new(".")))
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
        let payload = build_payload(task, workdir.as_deref())?;
        writeln!(
            log_file,
            "started_at={} command={} tool AcpExec {} --json",
            Utc::now().to_rfc3339(),
            self.maenifold_bin.display(),
            payload
        )
        .with_context(|| format!("failed to write log header '{}'", log_path.display()))?;
        log_file
            .sync_all()
            .with_context(|| format!("failed to sync log file '{}'", log_path.display()))?;
        let stdout = Stdio::from(
            log_file
                .try_clone()
                .with_context(|| format!("failed to clone log file '{}'", log_path.display()))?,
        );
        let stderr = Stdio::from(
            log_file
                .try_clone()
                .with_context(|| format!("failed to clone log file '{}'", log_path.display()))?,
        );
        let mut command = Command::new(&self.maenifold_bin);
        command
            .arg("tool")
            .arg("AcpExec")
            .arg(&payload)
            .arg("--json")
            .stdout(stdout)
            .stderr(stderr);
        if let Some(dir) = workdir.as_deref() {
            command.current_dir(dir);
        }
        configure_command(&mut command);
        let mut child = command.spawn().with_context(|| {
            format!(
                "failed to spawn maenifold for scheduled task '{}' using '{}'",
                task.id,
                self.maenifold_bin.display()
            )
        })?;
        let timeout = Duration::from_secs(u64::from(task.timeout_seconds));
        let started = Instant::now();
        loop {
            match child.try_wait().context("failed to poll scheduled task child")? {
                Some(status) if status.success() => return Ok((RunStatus::Success, None)),
                Some(status) => {
                    let detail = match status.code() {
                        Some(code) => format!("exit code: {code}"),
                        None => "process terminated by signal".to_string(),
                    };
                    return Ok((RunStatus::Failed, Some(format!("{detail} (log: {})", log_path.display()))));
                }
                None if started.elapsed() > timeout => {
                    terminate_child(&mut child)?;
                    return Ok((
                        RunStatus::Timeout,
                        Some(format!(
                            "timed out after {} seconds (log: {})",
                            task.timeout_seconds,
                            log_path.display()
                        )),
                    ));
                }
                None => thread::sleep(Duration::from_millis(500)),
            }
        }
    }
    fn lock_path(&self, task_id: &str) -> PathBuf {
        self.lock_dir.join(task_id).join(".lock")
    }
    fn log_path(&self, task_id: &str) -> PathBuf {
        let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        self.store.task_dir(task_id).join("logs").join(format!("{timestamp}.log"))
    }
}
struct LockGuard {
    path: PathBuf,
}
impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
fn build_payload(task: &ScheduledTask, workdir: Option<&Path>) -> Result<String> {
    let mut payload = json!({
        "agent": &task.agent,
        "prompt": &task.prompt,
        "mode": "prompt",
        "autoGrantPermissions": true,
        "timeoutSeconds": task.timeout_seconds,
    });
    if let Some(dir) = workdir {
        payload["cwd"] = json!(dir.display().to_string());
    }
    serde_json::to_string(&payload).context("failed to serialize AcpExec payload")
}
fn is_already_running(error: &anyhow::Error) -> bool {
    error.to_string().contains("is already running (pid ")
}
fn read_lock_pid(lock_path: &Path) -> Result<Option<i32>> {
    let content = match fs::read_to_string(lock_path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed to read lock '{}'", lock_path.display()));
        }
    };
    let trimmed = content.trim();
    Ok(if trimmed.is_empty() { None } else { trimmed.parse::<i32>().ok() })
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
#[cfg(unix)]
fn pid_is_alive(pid: i32) -> bool {
    let result = unsafe { libc::kill(pid, 0) };
    result == 0 || matches!(std::io::Error::last_os_error().raw_os_error(), Some(libc::EPERM))
}
#[cfg(not(unix))]
fn pid_is_alive(_pid: i32) -> bool {
    false
}
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

    child.kill().context("failed to kill timed out scheduled task child")?;
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
