#[cfg(unix)]
use std::{io, time::Duration};
#[cfg(unix)]
use tokio::time::{Instant, sleep};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    task::JoinHandle,
};

#[cfg(unix)]
const PROCESS_GROUP_TERM_WAIT: Duration = Duration::from_millis(100);
#[cfg(unix)]
const PROCESS_GROUP_POLL_INTERVAL: Duration = Duration::from_millis(10);

pub(super) fn configure_new_process_group(command: &mut tokio::process::Command) {
    #[cfg(unix)]
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

pub(super) fn log_io_task_exit<E: std::fmt::Display>(
    result: std::result::Result<std::result::Result<(), E>, tokio::task::JoinError>,
) {
    match result {
        Ok(Ok(())) => tracing::debug!("ACP I/O task exited"),
        Ok(Err(error)) => tracing::warn!("ACP I/O task exited: {error}"),
        Err(error) => tracing::warn!("ACP I/O task join failed: {error}"),
    }
}

pub(super) fn spawn_stderr_reader(stderr: tokio::process::ChildStderr) -> JoinHandle<()> {
    tokio::task::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            tracing::debug!("ACP stderr: {line}");
        }
    })
}

pub(super) async fn reap_child(
    child: &mut tokio::process::Child,
    stderr_task: Option<JoinHandle<()>>,
    io_task: Option<JoinHandle<std::result::Result<(), sacp::Error>>>,
) {
    if !kill_process_group(child).await
        && let Err(error) = child.start_kill()
    {
        tracing::debug!("Failed to kill ACP child: {error}");
    }
    let _ = child.wait().await;
    if let Some(task) = stderr_task {
        let _ = task.await;
    }
    if let Some(task) = io_task {
        log_io_task_exit(task.await);
    }
}

#[cfg(not(unix))]
async fn kill_process_group(_child: &mut tokio::process::Child) -> bool {
    false
}

#[cfg(unix)]
async fn kill_process_group(child: &mut tokio::process::Child) -> bool {
    let Some(leader_pid) = process_group_leader_pid(child) else {
        return false;
    };

    if let Err(error) = signal_process_group(leader_pid, libc::SIGTERM) {
        if error.raw_os_error() != Some(libc::ESRCH) {
            tracing::debug!("Failed to SIGTERM ACP process group {leader_pid}: {error}");
            return false;
        }
        return true;
    }

    if wait_for_child_exit(child, PROCESS_GROUP_TERM_WAIT).await {
        return true;
    }

    if let Err(error) = signal_process_group(leader_pid, libc::SIGKILL)
        && error.raw_os_error() != Some(libc::ESRCH)
    {
        tracing::debug!("Failed to SIGKILL ACP process group {leader_pid}: {error}");
        return false;
    }

    let _ = wait_for_child_exit(child, PROCESS_GROUP_TERM_WAIT).await;
    true
}

#[cfg(unix)]
fn process_group_leader_pid(child: &mut tokio::process::Child) -> Option<i32> {
    let leader_pid = child.id().and_then(|pid| i32::try_from(pid).ok())?;

    match child.try_wait() {
        Ok(Some(_)) => return None,
        Ok(None) => {}
        Err(error) => {
            tracing::debug!("Failed to inspect ACP child {leader_pid} before killpg: {error}");
            return None;
        }
    }

    match unsafe { libc::getpgid(leader_pid) } {
        -1 => {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::ESRCH) {
                tracing::debug!("Failed to query ACP process group for {leader_pid}: {error}");
            }
            None
        }
        pgid if pgid == leader_pid => Some(leader_pid),
        pgid => {
            tracing::debug!(
                "Skipping ACP process-group kill for pid {leader_pid}: expected leader-owned pgid, got {pgid}"
            );
            None
        }
    }
}

#[cfg(unix)]
fn signal_process_group(leader_pid: i32, signal: i32) -> io::Result<()> {
    let result = unsafe { libc::killpg(leader_pid, signal) };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(unix)]
async fn wait_for_child_exit(child: &mut tokio::process::Child, timeout_at: Duration) -> bool {
    let deadline = Instant::now() + timeout_at;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return true,
            Ok(None) => {}
            Err(error) => {
                tracing::debug!("Failed to wait for ACP child exit: {error}");
                return false;
            }
        }

        if Instant::now() >= deadline {
            return false;
        }

        sleep(PROCESS_GROUP_POLL_INTERVAL).await;
    }
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use super::{configure_new_process_group, reap_child};
    #[cfg(unix)]
    use std::{io, process::Stdio, time::Duration};
    #[cfg(unix)]
    use tokio::{
        io::{AsyncBufReadExt, BufReader},
        process::{Child, Command},
        time::{Instant, timeout},
    };

    #[cfg(unix)]
    fn process_exists(pid: u32) -> bool {
        let pid = match i32::try_from(pid) {
            Ok(pid) => pid,
            Err(_) => return false,
        };
        let result = unsafe { libc::kill(pid, 0) };
        result == 0 || io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
    }

    #[cfg(unix)]
    async fn wait_for_process_exit(pid: u32, timeout_at: Duration) -> bool {
        let deadline = Instant::now() + timeout_at;
        while Instant::now() < deadline {
            if !process_exists(pid) {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        !process_exists(pid)
    }

    #[cfg(unix)]
    fn kill_process(pid: u32) {
        if let Ok(pid) = i32::try_from(pid) {
            unsafe {
                libc::kill(pid, libc::SIGKILL);
            }
        }
    }

    #[cfg(unix)]
    async fn spawn_wrapped_descendant() -> (Child, u32) {
        let mut command = Command::new("sh");
        command
            .args(["-c", "sh -c 'exec sleep 30' & echo $!; wait"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        configure_new_process_group(&mut command);
        let mut child = command.spawn().expect("wrapped child should spawn");
        let stdout = child.stdout.take().expect("stdout should be piped");
        let mut lines = BufReader::new(stdout).lines();
        let pid = timeout(Duration::from_secs(1), lines.next_line())
            .await
            .expect("child should report descendant pid promptly")
            .expect("stdout read should succeed")
            .expect("child should report one descendant pid")
            .parse::<u32>()
            .expect("reported descendant pid should parse");
        assert!(process_exists(pid), "descendant pid {pid} should be alive");
        (child, pid)
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn reap_child_kills_wrapped_descendants() {
        let (mut child, descendant_pid) = spawn_wrapped_descendant().await;

        reap_child(&mut child, None, None).await;

        let descendant_reaped = wait_for_process_exit(descendant_pid, Duration::from_secs(2)).await;
        if !descendant_reaped {
            kill_process(descendant_pid);
            let _ = wait_for_process_exit(descendant_pid, Duration::from_secs(1)).await;
        }
        assert!(
            descendant_reaped,
            "wrapped descendant pid {descendant_pid} survived reap_child"
        );
    }
}
