use maenifold_core::json::OutputFormat;
use maenifold_core::tools::ToolContext;
use maenifold_core::workflow::pipeline::{
    abandon_active_nodes, execute_pipeline, finalize_failed_pipeline_execution,
    persist_final_pipeline_status, pipeline_lock_path,
};
use maenifold_core::{Config, MaenifoldError, Result, logging};
use std::io::{Seek, SeekFrom, Write};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() {
    let config = Config::current();
    if let Err(error) = config.ensure_directories() {
        eprintln!("ERROR: {error}");
        std::process::exit(1);
    }
    let guard = logging::init_tracing(&config);

    let code = match run(config).await {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("ERROR: {error}");
            1
        }
    };

    drop(guard);
    std::process::exit(code);
}

async fn run(config: Config) -> Result<()> {
    let (session_id, prompt) = parse_args()?;
    let context = ToolContext::new(config, OutputFormat::Json);
    let cancel = CancellationToken::new();

    // T-93.4: Acquire OS-level exclusive advisory lock on <session>.lock before executing.
    // Uses fd-lock (Cargo's flock.rs pattern): RAII guard released on process exit/death.
    // The lock file contains "{hostname}-{pid}" for operator visibility.
    let lock_path = pipeline_lock_path(&session_id, &context.config);
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
    lock_guard.seek(SeekFrom::Start(0))?;
    lock_guard.write_all(holder.as_bytes())?;
    lock_guard.flush()?;

    tracing::info!(
        session_id = %session_id,
        holder = %holder,
        "[pipeline-driver] acquired session lock"
    );

    // T-93.4: Crash recovery — abandon orphaned Active nodes from previous crash.
    // Since we just acquired the exclusive lock, any Active nodes have no running task.
    // Mark them abandoned so the reconciliation loop can proceed cleanly.
    let recovered = abandon_active_nodes(&context, &session_id)?;
    if recovered > 0 {
        tracing::warn!(
            session_id = %session_id,
            recovered = recovered,
            "[pipeline-driver] abandoned orphaned active nodes from previous crash"
        );
    }

    let signal_cancel = cancel.clone();
    let signal_session_id = session_id.clone();
    let signal_task = tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                tracing::warn!(
                    session_id = %signal_session_id,
                    "[pipeline-driver] received ctrl-c; cancelling pipeline"
                );
                signal_cancel.cancel();
            }
            Err(error) => {
                tracing::error!(
                    session_id = %signal_session_id,
                    error = %error,
                    "[pipeline-driver] failed to listen for ctrl-c"
                );
            }
        }
    });

    let execution_result = execute_pipeline(&context, &session_id, &prompt, cancel).await;
    signal_task.abort();
    let _ = signal_task.await;

    let result = match execution_result {
        Ok(execution_result) => {
            persist_final_pipeline_status(&context, &session_id, &execution_result.status)
        }
        Err(error) => {
            finalize_failed_pipeline_execution(&context, &session_id);
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
