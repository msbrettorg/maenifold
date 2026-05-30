#[cfg(unix)]
use std::time::Duration;

#[cfg(unix)]
const PROCESS_GROUP_TERM_WAIT: Duration = Duration::from_millis(200);

#[cfg(unix)]
pub(super) async fn terminate_process_group_id(process_group_id: Option<i32>) {
    let Some(process_group_id) = process_group_id else {
        return;
    };
    signal_process_group(process_group_id, libc::SIGTERM);
    tokio::time::sleep(PROCESS_GROUP_TERM_WAIT).await;
    signal_process_group(process_group_id, libc::SIGKILL);
}

#[cfg(not(unix))]
pub(super) async fn terminate_process_group_id(_process_group_id: Option<i32>) {}

#[cfg(unix)]
fn signal_process_group(process_group_id: i32, signal: i32) {
    let result = unsafe { libc::killpg(process_group_id, signal) };
    if result != 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::ESRCH) {
            tracing::warn!(
                process_group_id,
                signal,
                error = %error,
                "failed to signal MCP stdio process group"
            );
        }
    }
}
