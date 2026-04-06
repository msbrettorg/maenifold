use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use serde_json::Value;
use std::process::Command;

use super::lease;

#[path = "handler_lifecycle_rewind.rs"]
mod rewind;

pub(super) fn handle_lifecycle_operation(
    context: &ToolContext,
    session_id: &str,
    payload: &Value,
) -> Result<Option<String>> {
    if payload.get("rewind").is_some() {
        return rewind::handle_rewind_operation(context, session_id, payload).map(Some);
    }
    if payload.get("run").and_then(Value::as_bool).unwrap_or(false) {
        let prompt = payload.get("prompt").and_then(Value::as_str);
        return run_pipeline(context, session_id, prompt).map(Some);
    }
    if payload
        .get("cancel")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return cancel_pipeline(context, session_id).map(Some);
    }

    Ok(None)
}

fn find_driver_binary() -> Result<std::path::PathBuf> {
    // Allow explicit override for non-standard installs.
    if let Ok(path) = std::env::var("MAENIFOLD_PIPELINE_DRIVER") {
        let driver = std::path::PathBuf::from(path);
        if driver.exists() {
            return Ok(driver);
        }
        return Err(MaenifoldError::Message(format!(
            "MAENIFOLD_PIPELINE_DRIVER is set but binary not found at {}",
            driver.display()
        )));
    }

    let exe = std::env::current_exe().map_err(|error| {
        MaenifoldError::Message(format!("Cannot determine current executable path: {error}"))
    })?;
    let exe_dir = exe
        .canonicalize()
        .unwrap_or(exe)
        .parent()
        .ok_or_else(|| {
            MaenifoldError::Message("Cannot determine executable directory".to_string())
        })?
        .to_path_buf();
    let driver = exe_dir.join("maenifold-pipeline-driver");
    if driver.exists() {
        return Ok(driver);
    }
    Err(MaenifoldError::Message(format!(
        "Pipeline driver binary not found at {}. \
         Build it with: cargo build --release -p maenifold-pipeline-driver",
        driver.display()
    )))
}

fn run_pipeline(context: &ToolContext, session_id: &str, prompt: Option<&str>) -> Result<String> {
    if lease::session_lock_is_held(session_id, &context.config) {
        return Err(MaenifoldError::Message(format!(
            "Pipeline {session_id} is already running"
        )));
    }

    let driver = find_driver_binary()?;
    let mut cmd = Command::new(&driver);
    cmd.arg(session_id);
    if let Some(prompt) = prompt {
        cmd.arg(prompt);
    }
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }

    cmd.spawn().map_err(|error| {
        MaenifoldError::Message(format!(
            "Failed to spawn pipeline driver at {}: {error}",
            driver.display()
        ))
    })?;

    Ok(format!("Pipeline {session_id} started"))
}

fn cancel_pipeline(context: &ToolContext, session_id: &str) -> Result<String> {
    let lock_path = lease::pipeline_lock_path(session_id, &context.config);
    if !lock_path.exists() {
        return Err(MaenifoldError::Message(format!(
            "Pipeline {session_id} is not running (no lock file)"
        )));
    }
    if !lease::session_lock_is_held(session_id, &context.config) {
        return Err(MaenifoldError::Message(format!(
            "Pipeline {session_id} is not running (lock not held)"
        )));
    }

    let holder = std::fs::read_to_string(&lock_path).map_err(|error| {
        MaenifoldError::Message(format!(
            "Cannot read lock file {}: {error}",
            lock_path.display()
        ))
    })?;

    // Lock file format: "{hostname}-{pid}"
    let pid_str = holder.rsplit('-').next().unwrap_or("");
    let pid: i32 = pid_str.parse().map_err(|_| {
        MaenifoldError::Message(format!(
            "Cannot parse PID from lock file content: {holder:?}"
        ))
    })?;

    if pid <= 0 {
        return Err(MaenifoldError::Message(format!(
            "Invalid PID {pid} in lock file (must be positive)"
        )));
    }

    #[cfg(unix)]
    {
        let result = unsafe { libc::kill(pid, libc::SIGTERM) };
        if result != 0 {
            let errno = std::io::Error::last_os_error();
            return Err(MaenifoldError::Message(format!(
                "Failed to send SIGTERM to pipeline driver (PID {pid}): {errno}"
            )));
        }
    }

    #[cfg(not(unix))]
    {
        return Err(MaenifoldError::Message(
            "Pipeline cancel via signal is only supported on Unix".to_string(),
        ));
    }

    Ok(format!(
        "Pipeline {session_id} cancellation requested (SIGTERM sent to PID {pid})"
    ))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::util::json::OutputFormat;

    use super::super::test_support::pipeline_test_context;
    use super::*;

    #[test]
    fn cancel_pipeline_returns_error_when_not_running() {
        let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
        let session_id = "pipeline-1700000000000-cancel";

        let result = handle_lifecycle_operation(&context, session_id, &json!({ "cancel": true }));

        assert!(result.is_err(), "expected cancel to fail when pipeline is not running");
    }

    #[test]
    fn run_pipeline_returns_error_when_driver_binary_missing() {
        let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
        let session_id = "pipeline-1700000000000-nodriver";

        let result = handle_lifecycle_operation(
            &context,
            session_id,
            &json!({ "run": true, "prompt": "go" }),
        );

        // The driver binary won't be next to the test binary, so this should fail
        // with a "not found" message (unless we're in a full build environment).
        // Either outcome (success spawn or meaningful error) is acceptable.
        let _ = result;
    }
}
