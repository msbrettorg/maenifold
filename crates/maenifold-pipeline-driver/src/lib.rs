pub mod recovery;
pub mod recovery_preflight;

use maenifold_core::json::OutputFormat;
use maenifold_core::tools::ToolContext;
use maenifold_core::workflow::pipeline::{
    execute_pipeline, finalize_failed_pipeline_execution, persist_final_pipeline_status,
    pipeline_lock_path,
};
use maenifold_core::{Config, MaenifoldError, Result};
use std::io::{Seek, SeekFrom, Write};
use tokio_util::sync::CancellationToken;

pub async fn run(config: Config) -> Result<()> {
    let (session_id, prompt) = parse_args()?;
    run_pipeline_session(config, &session_id, &prompt).await
}

pub async fn run_pipeline_session(config: Config, session_id: &str, prompt: &str) -> Result<()> {
    let context = ToolContext::new(config, OutputFormat::Json);
    jockey::acp::set_config_root(context.config.maenifold_root.clone());
    let cancel = CancellationToken::new();

    // T-93.4: Acquire OS-level exclusive advisory lock on <session>.lock before executing.
    // Uses fd-lock (Cargo's flock.rs pattern): RAII guard released on process exit/death.
    // The lock file contains "{hostname}-{pid}" for operator visibility.
    let lock_path = pipeline_lock_path(session_id, &context.config);
    if let Some(parent) = lock_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            MaenifoldError::Message(format!(
                "Cannot create lock directory {}: {e}",
                parent.display()
            ))
        })?;
    }
    let lock_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|e| {
            MaenifoldError::Message(format!(
                "Cannot open lock file {}: {e}",
                lock_path.display()
            ))
        })?;
    let mut rw_lock = fd_lock::RwLock::new(lock_file);
    let mut lock_guard = rw_lock.try_write().map_err(|_| {
        MaenifoldError::Message(format!(
            "Pipeline {session_id} is already running (lock held at {}). \
             Kill the owner or wait for it to finish.",
            lock_path.display()
        ))
    })?;

    // Write holder info so operators can identify the owner process.
    let holder = format!("{}-{}", hostname_string(), std::process::id());
    lock_guard.set_len(0)?;
    lock_guard.seek(SeekFrom::Start(0))?;
    lock_guard.write_all(holder.as_bytes())?;
    lock_guard.flush()?;

    tracing::info!(
        session_id = %session_id,
        holder = %holder,
        "[pipeline-driver] acquired session lock"
    );

    let recovery = recovery::reconcile_driver_start(&context, session_id)?;
    if recovery.reset_to_pending > 0 {
        tracing::info!(
            session_id = %session_id,
            reset_to_pending = recovery.reset_to_pending,
            "[pipeline-driver] reset dispatched active nodes to pending for first driver start"
        );
    }
    if recovery.recovered > 0 {
        tracing::warn!(
            session_id = %session_id,
            recovered = recovery.recovered,
            "[pipeline-driver] recovered crashed active nodes from previous driver"
        );
    }

    let signal_cancel = cancel.clone();
    let signal_session_id = session_id.to_string();
    let signal_task = tokio::spawn(async move {
        wait_for_shutdown_signal(&signal_session_id).await;
        signal_cancel.cancel();
    });

    let execution_result = execute_pipeline(&context, session_id, prompt, cancel, None).await;
    signal_task.abort();
    let _ = signal_task.await;

    let result = match execution_result {
        Ok(mut execution_result) => {
            if recovery_preflight::promote_preflight_env_failures_to_stuck(&context, session_id)?
                > 0
            {
                execution_result.status = "stuck".to_string();
            }
            persist_final_pipeline_status(&context, session_id, &execution_result.status)
        }
        Err(error) => {
            // Defensive: promote any persisted preflight failures before finalizing
            // so the recovery surface is consistent for Ok and Err outcomes.
            let _ =
                recovery_preflight::promote_preflight_env_failures_to_stuck(&context, session_id);
            finalize_failed_pipeline_execution(&context, session_id);
            Err(error)
        }
    };

    // Lock guard held through all final writes; dropping releases the OS lock.
    drop(lock_guard);
    result
}

fn parse_args() -> Result<(String, String)> {
    let mut args = std::env::args().skip(1);
    let session_id = args.next().ok_or_else(|| {
        MaenifoldError::Message(
            "usage: maenifold-pipeline-driver <session-id> [prompt ...]".to_string(),
        )
    })?;
    let prompt = args.collect::<Vec<_>>().join(" ");
    Ok((session_id, prompt))
}

/// Returns the machine hostname for the lock-file holder string.
/// Falls back to "unknown" if the hostname cannot be determined.
fn hostname_string() -> String {
    let mut buf = [0u8; 256];
    let result = unsafe { libc::gethostname(buf.as_mut_ptr() as *mut libc::c_char, buf.len()) };
    if result != 0 {
        return "unknown".to_string();
    }
    let nul = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..nul]).into_owned()
}

async fn wait_for_shutdown_signal(session_id: &str) {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut sigterm = signal(SignalKind::terminate()).expect("failed to register SIGTERM");
        let mut sigint = signal(SignalKind::interrupt()).expect("failed to register SIGINT");
        tokio::select! {
            _ = sigterm.recv() => {
                tracing::warn!(session_id = %session_id, "[pipeline-driver] received SIGTERM; cancelling");
            }
            _ = sigint.recv() => {
                tracing::warn!(session_id = %session_id, "[pipeline-driver] received SIGINT; cancelling");
            }
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await.ok();
        tracing::warn!(session_id = %session_id, "[pipeline-driver] received ctrl-c; cancelling");
    }
}
