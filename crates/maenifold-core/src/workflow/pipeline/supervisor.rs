use crate::error::{MaenifoldError, Result};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

static SUPERVISOR: Lazy<Mutex<HashMap<String, RunHandle>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub(super) struct RunHandle {
    pub pid: u32,
    child: Child,
    pub started_at: Instant,
    pub attempts: u32,
}

pub(super) fn supervisor_run(root: &Path, session_id: &str, prompt: &str) -> Result<u32> {
    let script_path = root.join("maenifold-acp");
    let mut command = Command::new("bash");
    command
        .arg(&script_path)
        .arg("pipeline")
        .arg(session_id)
        .arg(prompt)
        .current_dir(root)
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    spawn_supervised_command(session_id, command)
}

pub(super) fn supervisor_cancel(session_id: &str) -> Result<()> {
    let mut handle = SUPERVISOR.lock().remove(session_id).ok_or_else(|| {
        MaenifoldError::InvalidArgument(format!("Pipeline {session_id} isn't running"))
    })?;

    let _ = send_signal(handle.pid, libc::SIGTERM)?;
    if !wait_for_exit(&mut handle.child, Duration::from_secs(5))? {
        let _ = send_signal(handle.pid, libc::SIGKILL)?;
        let _ = wait_for_exit(&mut handle.child, Duration::from_secs(1))?;
    }

    Ok(())
}

pub(super) fn supervisor_status(session_id: &str) -> Option<(u32, Instant, u32)> {
    let guard = SUPERVISOR.lock();
    guard
        .get(session_id)
        .map(|handle| (handle.pid, handle.started_at, handle.attempts))
}

pub(super) fn supervisor_is_running(session_id: &str) -> bool {
    supervisor_status(session_id).is_some()
}

#[doc(hidden)]
pub(super) fn supervisor_clear_all() {
    let handles = {
        let mut guard = SUPERVISOR.lock();
        std::mem::take(&mut *guard)
    };

    for (_, mut handle) in handles {
        let _ = send_signal(handle.pid, libc::SIGKILL);
        let _ = wait_for_exit(&mut handle.child, Duration::from_secs(1));
    }
}

fn spawn_supervised_command(session_id: &str, mut command: Command) -> Result<u32> {
    let mut guard = SUPERVISOR.lock();
    if guard.contains_key(session_id) {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Pipeline {session_id} is already running"
        )));
    }

    let child = command.spawn()?;
    let pid = child.id();
    guard.insert(
        session_id.to_string(),
        RunHandle {
            pid,
            child,
            started_at: Instant::now(),
            attempts: 0,
        },
    );
    drop(guard);

    spawn_monitor_thread(session_id.to_string(), pid);
    Ok(pid)
}

fn spawn_monitor_thread(session_id: String, pid: u32) {
    thread::spawn(move || {
        loop {
            match poll_process_exit(pid) {
                ProcessState::Running => thread::sleep(Duration::from_millis(100)),
                ProcessState::Exited | ProcessState::Gone => {
                    let mut guard = SUPERVISOR.lock();
                    let remove_entry = guard
                        .get(&session_id)
                        .map(|handle| handle.pid == pid)
                        .unwrap_or(false);
                    if remove_entry {
                        guard.remove(&session_id);
                    }
                    break;
                }
            }
        }
    });
}

enum ProcessState {
    Running,
    Exited,
    Gone,
}

fn poll_process_exit(pid: u32) -> ProcessState {
    loop {
        let mut status = 0;
        let result = unsafe { libc::waitpid(pid as i32, &mut status, libc::WNOHANG) };
        if result == 0 {
            return ProcessState::Running;
        }
        if result == pid as i32 {
            return ProcessState::Exited;
        }

        let error = std::io::Error::last_os_error();
        match error.raw_os_error() {
            Some(code) if code == libc::EINTR => continue,
            Some(code) if code == libc::ECHILD => return ProcessState::Gone,
            _ => return ProcessState::Gone,
        }
    }
}

fn send_signal(pid: u32, signal: i32) -> Result<bool> {
    loop {
        let result = unsafe { libc::kill(pid as i32, signal) };
        if result == 0 {
            return Ok(true);
        }

        let error = std::io::Error::last_os_error();
        match error.raw_os_error() {
            Some(code) if code == libc::EINTR => continue,
            Some(code) if code == libc::ESRCH => return Ok(false),
            _ => return Err(error.into()),
        }
    }
}

fn wait_for_exit(child: &mut Child, timeout: Duration) -> Result<bool> {
    let deadline = Instant::now() + timeout;
    loop {
        if child_has_exited(child)? {
            return Ok(true);
        }
        if Instant::now() >= deadline {
            return Ok(false);
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn child_has_exited(child: &mut Child) -> Result<bool> {
    match child.try_wait() {
        Ok(Some(_)) => Ok(true),
        Ok(None) => Ok(false),
        Err(error) if error.raw_os_error() == Some(libc::ECHILD) => Ok(true),
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
pub(super) fn supervisor_spawn_test_process(
    session_id: &str,
    program: &str,
    args: &[&str],
) -> Result<u32> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    spawn_supervised_command(session_id, command)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    struct SupervisorReset;

    impl Drop for SupervisorReset {
        fn drop(&mut self) {
            supervisor_clear_all();
        }
    }

    fn reset_supervisor() -> SupervisorReset {
        supervisor_clear_all();
        SupervisorReset
    }

    #[test]
    #[serial(config)]
    fn supervisor_run_spawns_process_and_registers() {
        let _reset = reset_supervisor();

        let pid = supervisor_spawn_test_process("pipeline-test-run", "sleep", &["30"])
            .expect("spawn sleep process");

        let (status_pid, _started_at, attempts) =
            supervisor_status("pipeline-test-run").expect("registered run handle");
        assert_eq!(status_pid, pid);
        assert_eq!(attempts, 0);
        assert!(supervisor_is_running("pipeline-test-run"));

        supervisor_cancel("pipeline-test-run").expect("cancel sleep process");
    }

    #[test]
    #[serial(config)]
    fn supervisor_run_rejects_already_running() {
        let _reset = reset_supervisor();
        supervisor_spawn_test_process("pipeline-test-duplicate", "sleep", &["30"])
            .expect("spawn sleep process");

        let error = supervisor_spawn_test_process("pipeline-test-duplicate", "sleep", &["30"])
            .expect_err("second run should fail");
        assert!(error.to_string().contains("already running"));
    }

    #[test]
    #[serial(config)]
    fn supervisor_cancel_terminates_and_removes() {
        let _reset = reset_supervisor();
        supervisor_spawn_test_process("pipeline-test-cancel", "sleep", &["30"])
            .expect("spawn sleep process");

        supervisor_cancel("pipeline-test-cancel").expect("cancel sleep process");

        assert!(!supervisor_is_running("pipeline-test-cancel"));
        assert!(supervisor_status("pipeline-test-cancel").is_none());
    }

    #[test]
    #[serial(config)]
    fn supervisor_status_returns_none_when_not_running() {
        let _reset = reset_supervisor();
        assert!(supervisor_status("pipeline-test-missing").is_none());
        assert!(!supervisor_is_running("pipeline-test-missing"));
    }

    #[test]
    #[serial(config)]
    fn supervisor_process_exit_auto_cleans_up() {
        let _reset = reset_supervisor();
        supervisor_spawn_test_process("pipeline-test-exit", "sleep", &["1"])
            .expect("spawn sleep process");

        let deadline = Instant::now() + Duration::from_secs(3);
        while supervisor_is_running("pipeline-test-exit") && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(100));
        }

        assert!(!supervisor_is_running("pipeline-test-exit"));
        assert!(supervisor_status("pipeline-test-exit").is_none());
    }
}
