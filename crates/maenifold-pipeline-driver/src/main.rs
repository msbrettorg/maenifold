use chrono::{Datelike, SecondsFormat, TimeZone, Utc};
use maenifold_core::json::OutputFormat;
use maenifold_core::markdown::{read_markdown, write_markdown};
use maenifold_core::tools::ToolContext;
use maenifold_core::workflow::pipeline::{
    NodeState, NodeStatus, PipelineDefinition, compute_pipeline_status,
    compute_pipeline_status_with_definition, execute_pipeline, finalize_failed_pipeline_execution,
    persist_final_pipeline_status, pipeline_lock_path, recover_crashed_nodes,
};
use maenifold_core::{Config, MaenifoldError, Result, logging};
use serde_yaml::Value as YamlValue;
use std::collections::HashMap;
use std::io::{Seek, SeekFrom, Write};
use tokio_util::sync::CancellationToken;
use tracing::{error, warn};

#[tokio::main]
async fn main() {
    let config = Config::current();
    if let Err(error) = config.ensure_directories() {
        eprintln!("failed to ensure pipeline driver directories: {error}");
        std::process::exit(1);
    }
    let guard = logging::init_tracing(&config);

    let code = match run(config).await {
        Ok(()) => 0,
        Err(error) => {
            error!(error = %error, "pipeline driver execution failed");
            1
        }
    };

    drop(guard);
    std::process::exit(code);
}

async fn run(config: Config) -> Result<()> {
    let (session_id, prompt) = parse_args()?;
    let context = ToolContext::new(config, OutputFormat::Json);
    jockey::acp::set_config_root(context.config.maenifold_root.clone());
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
    lock_guard.set_len(0)?;
    lock_guard.seek(SeekFrom::Start(0))?;
    lock_guard.write_all(holder.as_bytes())?;
    lock_guard.flush()?;

    tracing::info!(
        session_id = %session_id,
        holder = %holder,
        "[pipeline-driver] acquired session lock"
    );

    let recovery = reconcile_driver_start(&context, &session_id)?;
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
    let signal_session_id = session_id.clone();
    let signal_task = tokio::spawn(async move {
        wait_for_shutdown_signal(&signal_session_id).await;
        signal_cancel.cancel();
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

#[derive(Debug, Default)]
struct DriverRecovery {
    reset_to_pending: usize,
    recovered: usize,
}

enum DriverStartedAtState {
    Missing,
    Present,
    Malformed,
}

fn reconcile_driver_start(context: &ToolContext, session_id: &str) -> Result<DriverRecovery> {
    let state = read_driver_started_at_state(context, session_id)?;
    let mut recovery = DriverRecovery::default();

    match state {
        DriverStartedAtState::Missing => {
            recovery.reset_to_pending = reset_active_nodes_to_pending(context, session_id)?;
        }
        DriverStartedAtState::Present => {
            recovery.recovered = recover_crashed_nodes(context, session_id)?;
        }
        DriverStartedAtState::Malformed => {
            warn!(
                session_id = %session_id,
                "[pipeline-driver] driverStartedAt frontmatter is malformed; failing closed"
            );
            recovery.recovered = recover_crashed_nodes(context, session_id)?;
        }
    }

    write_driver_started_at(context, session_id)?;
    Ok(recovery)
}

fn read_driver_started_at_state(
    context: &ToolContext,
    session_id: &str,
) -> Result<DriverStartedAtState> {
    let pipeline_uri = pipeline_session_uri(session_id);
    let (frontmatter, _, _) = read_markdown(&pipeline_uri, &context.config)?;
    let frontmatter = frontmatter.ok_or_else(|| {
        MaenifoldError::Message(format!(
            "Pipeline session {session_id} has invalid frontmatter"
        ))
    })?;

    Ok(match frontmatter.get("driverStartedAt") {
        None => DriverStartedAtState::Missing,
        Some(YamlValue::String(_)) => DriverStartedAtState::Present,
        Some(_) => DriverStartedAtState::Malformed,
    })
}

fn reset_active_nodes_to_pending(context: &ToolContext, session_id: &str) -> Result<usize> {
    let pipeline_uri = pipeline_session_uri(session_id);
    let (frontmatter, pipeline_content, _) = read_markdown(&pipeline_uri, &context.config)?;
    let mut frontmatter = frontmatter.ok_or_else(|| {
        MaenifoldError::Message(format!(
            "Pipeline session {session_id} has invalid frontmatter"
        ))
    })?;
    let mut node_states = pipeline_node_states(&frontmatter)?;
    let mut reset_to_pending = 0usize;

    for (node_id, state) in &mut node_states {
        if !matches!(state.status, NodeStatus::Active) {
            continue;
        }
        tracing::info!(
            session_id = %session_id,
            node_id = %node_id,
            "resetting dispatched active node to pending for first driver start"
        );
        state.status = NodeStatus::Pending;
        state.failure_reason = None;
        update_workflow_status(context, &state.session_id, "pending")?;
        reset_to_pending += 1;
    }

    persist_pipeline_frontmatter(
        context,
        &pipeline_uri,
        &mut frontmatter,
        &pipeline_content,
        &node_states,
    )?;
    Ok(reset_to_pending)
}

fn write_driver_started_at(context: &ToolContext, session_id: &str) -> Result<()> {
    let pipeline_uri = pipeline_session_uri(session_id);
    let (frontmatter, pipeline_content, _) = read_markdown(&pipeline_uri, &context.config)?;
    let mut frontmatter = frontmatter.ok_or_else(|| {
        MaenifoldError::Message(format!(
            "Pipeline session {session_id} has invalid frontmatter"
        ))
    })?;
    frontmatter.insert(
        "driverStartedAt".to_string(),
        YamlValue::String(Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)),
    );
    write_markdown(
        &pipeline_uri,
        Some(&frontmatter),
        &pipeline_content,
        &context.config,
    )
}

fn persist_pipeline_frontmatter(
    context: &ToolContext,
    pipeline_uri: &str,
    frontmatter: &mut maenifold_core::markdown::Frontmatter,
    pipeline_content: &str,
    node_states: &HashMap<String, NodeState>,
) -> Result<()> {
    frontmatter.insert("nodeStates".to_string(), serde_yaml::to_value(node_states)?);
    let status = if let Some(definition) = frontmatter
        .get("definition")
        .cloned()
        .and_then(|value| serde_yaml::from_value::<PipelineDefinition>(value).ok())
    {
        compute_pipeline_status_with_definition(&definition, node_states).to_string()
    } else {
        compute_pipeline_status(node_states).to_string()
    };
    frontmatter.insert("status".to_string(), YamlValue::String(status));
    write_markdown(
        pipeline_uri,
        Some(frontmatter),
        pipeline_content,
        &context.config,
    )
}

fn pipeline_node_states(
    frontmatter: &maenifold_core::markdown::Frontmatter,
) -> Result<HashMap<String, NodeState>> {
    let node_states = frontmatter.get("nodeStates").cloned().ok_or_else(|| {
        MaenifoldError::Message("ERROR: Pipeline session missing frontmatter 'nodeStates'".into())
    })?;
    serde_yaml::from_value(node_states).map_err(Into::into)
}

fn update_workflow_status(
    context: &ToolContext,
    workflow_session_id: &str,
    status: &str,
) -> Result<()> {
    let workflow_uri = workflow_session_uri(workflow_session_id);
    let (frontmatter, workflow_content, _) = read_markdown(&workflow_uri, &context.config)?;
    let mut frontmatter = frontmatter.ok_or_else(|| {
        MaenifoldError::Message(format!(
            "Workflow session {workflow_session_id} has invalid frontmatter"
        ))
    })?;
    frontmatter.insert("status".to_string(), YamlValue::String(status.to_string()));
    write_markdown(
        &workflow_uri,
        Some(&frontmatter),
        &workflow_content,
        &context.config,
    )
}

fn pipeline_session_uri(session_id: &str) -> String {
    let (year, month, day) = session_date_parts(session_id);
    format!("memory://pipelines/{year:04}/{month:02}/{day:02}/{session_id}")
}

fn workflow_session_uri(session_id: &str) -> String {
    let (year, month, day) = session_date_parts(session_id);
    format!("memory://thinking/workflow/{year:04}/{month:02}/{day:02}/{session_id}")
}

fn session_date_parts(session_id: &str) -> (i32, u32, u32) {
    let timestamp = session_timestamp_millis(session_id)
        .and_then(|millis| Utc.timestamp_millis_opt(millis).single())
        .unwrap_or_else(Utc::now);
    (timestamp.year(), timestamp.month(), timestamp.day())
}

fn session_timestamp_millis(session_id: &str) -> Option<i64> {
    if session_id
        .split('-')
        .any(|segment| segment.trim().is_empty())
    {
        return None;
    }

    for segment in session_id.rsplit('-') {
        if segment.len() == 13
            && segment.bytes().all(|byte| byte.is_ascii_digit())
            && let Ok(millis) = segment.parse::<i64>()
            && Utc.timestamp_millis_opt(millis).single().is_some()
        {
            return Some(millis);
        }
    }

    None
}
