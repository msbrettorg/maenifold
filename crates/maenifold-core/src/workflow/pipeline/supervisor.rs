use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use crate::markdown::read_markdown;
use crate::sessions::{pipeline_session_uri, workflow_session_uri};
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use super::session_io::{
    frontmatter_value, persist_pipeline_state_with_status, update_workflow_node_frontmatter,
};
use super::{NodeState, NodeStatus};

static SUPERVISOR: Lazy<Mutex<HashMap<String, RunHandle>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
const MAX_RESTART_ATTEMPTS: u32 = 3;

pub(super) struct RunHandle {
    pub pid: u32,
    child: Child,
    pub started_at: Instant,
    pub attempts: u32,
    root: PathBuf,
    session_id_clone: String,
    prompt: String,
}

pub(super) fn supervisor_run(root: &Path, session_id: &str, prompt: &str) -> Result<u32> {
    spawn_supervised(session_id, root, prompt)
}

pub(super) fn supervisor_cancel(session_id: &str) -> Result<()> {
    let mut handle = SUPERVISOR.lock().remove(session_id).ok_or_else(|| {
        MaenifoldError::InvalidArgument(format!("Pipeline {session_id} isn't running"))
    })?;

    // Signal the process GROUP (negative PID) so the entire tree is terminated,
    // not just the bash wrapper. process_group(0) in spawn makes PGID == PID.
    let _ = send_signal_group(handle.pid, libc::SIGTERM)?;
    if !wait_for_exit(&mut handle.child, Duration::from_secs(5))? {
        let _ = send_signal_group(handle.pid, libc::SIGKILL)?;
        let _ = wait_for_exit(&mut handle.child, Duration::from_secs(1))?;
    }

    Ok(())
}

pub(super) fn supervisor_pause(session_id: &str) -> Result<u32> {
    #[cfg(not(unix))]
    {
        let _ = session_id;
        return Err(MaenifoldError::Message(
            "Pipeline pause is not supported on this platform".to_string(),
        ));
    }

    #[cfg(unix)]
    {
        let guard = SUPERVISOR.lock();
        let handle = guard.get(session_id).ok_or_else(|| {
            MaenifoldError::InvalidArgument(format!("Pipeline {session_id} isn't running"))
        })?;
        send_signal_group(handle.pid, libc::SIGSTOP)?;
        Ok(handle.pid)
    }
}

pub(super) fn supervisor_resume(session_id: &str) -> Result<u32> {
    #[cfg(not(unix))]
    {
        let _ = session_id;
        return Err(MaenifoldError::Message(
            "Pipeline resume is not supported on this platform".to_string(),
        ));
    }

    #[cfg(unix)]
    {
        let guard = SUPERVISOR.lock();
        let handle = guard.get(session_id).ok_or_else(|| {
            MaenifoldError::InvalidArgument(format!("Pipeline {session_id} isn't running"))
        })?;
        send_signal_group(handle.pid, libc::SIGCONT)?;
        Ok(handle.pid)
    }
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
        let _ = send_signal_group(handle.pid, libc::SIGKILL);
        let _ = wait_for_exit(&mut handle.child, Duration::from_secs(1));
    }
}

fn spawn_supervised(session_id: &str, root: &Path, prompt: &str) -> Result<u32> {
    let mut guard = SUPERVISOR.lock();
    if guard.contains_key(session_id) {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Pipeline {session_id} is already running"
        )));
    }

    let mut command = pipeline_command(root, session_id, prompt);
    let child = command.spawn()?;
    let pid = child.id();
    guard.insert(
        session_id.to_string(),
        RunHandle {
            pid,
            child,
            started_at: Instant::now(),
            attempts: 0,
            root: root.to_path_buf(),
            session_id_clone: session_id.to_string(),
            prompt: prompt.to_string(),
        },
    );
    drop(guard);

    spawn_monitor_thread(
        session_id.to_string(),
        pid,
        root.to_path_buf(),
        prompt.to_string(),
    );
    Ok(pid)
}

fn pipeline_command(root: &Path, session_id: &str, prompt: &str) -> Command {
    let mut command = Command::new("bash");
    command
        .arg(root.join("maenifold-acp"))
        .arg("pipeline")
        .arg(session_id)
        .arg(prompt)
        .current_dir(root)
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    command
}

fn spawn_monitor_thread(session_id: String, pid: u32, root: PathBuf, prompt: String) {
    thread::spawn(move || {
        loop {
            match poll_process_exit(pid) {
                ProcessState::Running => thread::sleep(Duration::from_millis(100)),
                ProcessState::ExitedClean | ProcessState::Gone => {
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
                ProcessState::ExitedCrash => {
                    let has_non_terminal_nodes =
                        pipeline_has_non_terminal_nodes(root.as_path(), &session_id);
                    let mut should_mark_stuck = false;

                    let restart_spawn_result = {
                        let mut guard = SUPERVISOR.lock();
                        let Some(handle) = guard.get_mut(&session_id) else {
                            break;
                        };

                        if handle.pid != pid {
                            break;
                        }

                        if has_non_terminal_nodes && handle.attempts < MAX_RESTART_ATTEMPTS {
                            handle.attempts += 1;
                            let attempts = handle.attempts;
                            let restart_root = handle.root.clone();
                            let restart_prompt = handle.prompt.clone();
                            let restart_session_id = handle.session_id_clone.clone();
                            let mut command = pipeline_command(
                                restart_root.as_path(),
                                &restart_session_id,
                                &restart_prompt,
                            );

                            match command.spawn() {
                                Ok(new_child) => {
                                    let new_pid = new_child.id();
                                    eprintln!(
                                        "[pipeline-supervisor] restarting {} (attempt {}/{}), new pid {}",
                                        session_id, attempts, MAX_RESTART_ATTEMPTS, new_pid
                                    );
                                    handle.pid = new_pid;
                                    handle.child = new_child;
                                    handle.started_at = Instant::now();
                                    Some(Ok(new_pid))
                                }
                                Err(err) => {
                                    eprintln!(
                                        "[pipeline-supervisor] restart failed for {}: {}",
                                        session_id, err
                                    );
                                    guard.remove(&session_id);
                                    Some(Err(err))
                                }
                            }
                        } else {
                            should_mark_stuck =
                                has_non_terminal_nodes && handle.attempts >= MAX_RESTART_ATTEMPTS;
                            if should_mark_stuck {
                                eprintln!(
                                    "[pipeline-supervisor] {} exhausted {} restart attempts, marking stuck",
                                    session_id, MAX_RESTART_ATTEMPTS
                                );
                            }
                            guard.remove(&session_id);
                            None
                        }
                    };

                    if let Some(Ok(new_pid)) = restart_spawn_result {
                        spawn_monitor_thread(session_id, new_pid, root, prompt);
                        return;
                    }

                    if should_mark_stuck {
                        mark_pipeline_stuck(root.as_path(), &session_id);
                    }
                    break;
                }
            }
        }
    });
}

enum ProcessState {
    Running,
    ExitedClean,
    ExitedCrash,
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
            if libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 0 {
                return ProcessState::ExitedClean;
            }
            return ProcessState::ExitedCrash;
        }

        let error = std::io::Error::last_os_error();
        match error.raw_os_error() {
            Some(code) if code == libc::EINTR => continue,
            Some(code) if code == libc::ECHILD => return ProcessState::Gone,
            _ => return ProcessState::Gone,
        }
    }
}

fn pipeline_has_non_terminal_nodes(root: &Path, session_id: &str) -> bool {
    let context = supervisor_context(root);
    let uri = pipeline_session_uri(session_id);
    let Ok((frontmatter, _, _)) = read_markdown(&uri, &context.config) else {
        return false;
    };
    let Some(frontmatter) = frontmatter else {
        return false;
    };
    let Ok(node_states) =
        frontmatter_value::<HashMap<String, NodeState>>(&frontmatter, "nodeStates")
    else {
        return false;
    };

    node_states.values().any(|state| {
        matches!(
            state.status,
            NodeStatus::Pending | NodeStatus::Ready | NodeStatus::Active
        )
    })
}

fn mark_pipeline_stuck(root: &Path, session_id: &str) {
    let context = supervisor_context(root);
    let uri = pipeline_session_uri(session_id);
    let Ok((frontmatter, pipeline_content, _)) = read_markdown(&uri, &context.config) else {
        eprintln!(
            "[pipeline-supervisor] failed to read pipeline session {} while marking stuck",
            session_id
        );
        return;
    };
    let Some(mut frontmatter) = frontmatter else {
        eprintln!(
            "[pipeline-supervisor] pipeline session {} has invalid frontmatter while marking stuck",
            session_id
        );
        return;
    };
    let Ok(mut node_states) =
        frontmatter_value::<HashMap<String, NodeState>>(&frontmatter, "nodeStates")
    else {
        eprintln!(
            "[pipeline-supervisor] pipeline session {} is missing nodeStates while marking stuck",
            session_id
        );
        return;
    };

    for state in node_states.values_mut() {
        if matches!(
            state.status,
            NodeStatus::Pending | NodeStatus::Ready | NodeStatus::Active
        ) {
            state.status = NodeStatus::Stuck;
            if let Err(err) =
                update_workflow_node_frontmatter(&context, &state.session_id, "stuck", None)
            {
                eprintln!(
                    "[pipeline-supervisor] failed to update workflow {} for stuck pipeline {}: {}",
                    state.session_id, session_id, err
                );
            }
        }
    }

    if let Err(err) = persist_pipeline_state_with_status(
        &mut frontmatter,
        &node_states,
        Some("stuck"),
        &uri,
        &pipeline_content,
        &context,
    ) {
        eprintln!(
            "[pipeline-supervisor] failed to persist stuck state for {}: {}",
            session_id, err
        );
    }
}

fn supervisor_context(root: &Path) -> ToolContext {
    let memory_path = root.join("memory");
    let thinking_path = memory_path.join("thinking");
    ToolContext::new(
        Config {
            maenifold_root: root.to_path_buf(),
            memory_path: memory_path.clone(),
            assets_path: root.join("assets"),
            thinking_path: thinking_path.clone(),
            sequential_path: thinking_path.join("sequential"),
            database_path: root.join("memory.db"),
        },
        OutputFormat::Json,
    )
}

/// Signal the entire process group (PGID == PID because we set process_group(0) on spawn).
fn send_signal_group(pid: u32, signal: i32) -> Result<bool> {
    loop {
        // Negative PID signals the process group, not just the leader
        let result = unsafe { libc::kill(-(pid as i32), signal) };
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
    let mut guard = SUPERVISOR.lock();
    if guard.contains_key(session_id) {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Pipeline {session_id} is already running"
        )));
    }

    let mut command = Command::new(program);
    command
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
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
            root: PathBuf::from("/tmp"),
            session_id_clone: session_id.to_string(),
            prompt: String::new(),
        },
    );
    drop(guard);

    spawn_monitor_thread(
        session_id.to_string(),
        pid,
        PathBuf::from("/tmp"),
        String::new(),
    );
    Ok(pid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::read_markdown;
    use crate::util::json::OutputFormat;
    use crate::workflow::pipeline::test_support::{
        pipeline_test_context, seed_pipeline_session_with_workflows,
    };
    use crate::workflow::pipeline::{PipelineDefinition, PipelineNode};
    use serde_yaml::Value as YamlValue;
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

    #[cfg(unix)]
    #[test]
    #[serial(config)]
    fn pipeline_pause_supervisor_sends_sigstop() {
        let _reset = reset_supervisor();

        let pid = supervisor_spawn_test_process("pipeline-test-pause", "sleep", &["30"])
            .expect("spawn process");
        assert!(pid > 0);

        let paused_pid = supervisor_pause("pipeline-test-pause").expect("pause");
        assert_eq!(paused_pid, pid);
        assert!(supervisor_is_running("pipeline-test-pause"));

        let resumed_pid = supervisor_resume("pipeline-test-pause").expect("resume");
        assert_eq!(resumed_pid, pid);
        assert!(supervisor_is_running("pipeline-test-pause"));

        supervisor_cancel("pipeline-test-pause").expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    #[serial(config)]
    fn pipeline_pause_supervisor_rejects_not_running() {
        let _reset = reset_supervisor();
        let error = supervisor_pause("nonexistent").expect_err("should fail");
        assert!(error.to_string().contains("isn't running"));
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

    #[test]
    #[serial(config)]
    fn supervisor_restart_increments_attempts_on_crash() {
        let _reset = reset_supervisor();
        supervisor_spawn_test_process("pipeline-test-crash", "false", &[]).expect("spawn process");

        let deadline = Instant::now() + Duration::from_secs(3);
        while supervisor_is_running("pipeline-test-crash") && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(100));
        }

        assert!(!supervisor_is_running("pipeline-test-crash"));
    }

    #[test]
    #[serial(config)]
    fn supervisor_clean_exit_removes_without_restart() {
        let _reset = reset_supervisor();
        supervisor_spawn_test_process("pipeline-test-clean", "true", &[]).expect("spawn process");

        let deadline = Instant::now() + Duration::from_secs(3);
        while supervisor_is_running("pipeline-test-clean") && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(100));
        }

        assert!(!supervisor_is_running("pipeline-test-clean"));
    }

    #[test]
    #[serial(config)]
    fn supervisor_marks_pipeline_stuck_when_retries_exhausted() {
        let (_tmp, context, _config_reset) = pipeline_test_context(OutputFormat::Json);
        let _reset = reset_supervisor();
        let pipeline_id = "pipeline-1735689600999";
        let definition = PipelineDefinition {
            nodes: vec![PipelineNode {
                id: "root".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: None,
            }],
            edges: vec![],
        };
        let node_states = HashMap::from([(
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689600999-root".to_string(),
                status: NodeStatus::Active,
                attempts: 0,
            },
        )]);
        seed_pipeline_session_with_workflows(&context, pipeline_id, definition, node_states, 1);

        mark_pipeline_stuck(context.config.maenifold_root.as_path(), pipeline_id);

        let (frontmatter, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline");
        let frontmatter = frontmatter.expect("pipeline frontmatter");
        assert_eq!(
            frontmatter.get("status").and_then(YamlValue::as_str),
            Some("stuck")
        );

        let (workflow_frontmatter, _, _) = read_markdown(
            &workflow_session_uri("workflow-1735689600999-root"),
            &context.config,
        )
        .expect("read workflow");
        let workflow_frontmatter = workflow_frontmatter.expect("workflow frontmatter");
        assert_eq!(
            workflow_frontmatter
                .get("status")
                .and_then(YamlValue::as_str),
            Some("stuck")
        );
    }
}
