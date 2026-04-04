use tokio::{
    io::{AsyncBufReadExt, BufReader},
    task::JoinHandle,
};

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
    if let Err(error) = child.start_kill() {
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
