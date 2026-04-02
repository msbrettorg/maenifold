use crate::error::{MaenifoldError, Result};
use std::path::Path;
use std::time::Instant;
use tokio::sync::{oneshot, watch};
use tokio_util::sync::CancellationToken;

use super::executor;
use super::supervisor::{
    RUNNING_PIPELINES, TRACKED_PIPELINES, TrackedPipeline, finish_pipeline, supervisor_context,
    supervisor_runtime,
};
use super::supervisor_detection::{
    mark_pipeline_stuck, pipeline_has_non_terminal_nodes, supervisor_detect_orphans_excluding,
};

pub(super) fn supervisor_run(root: &Path, session_id: &str, prompt: &str) -> Result<()> {
    let _ = supervisor_detect_orphans_excluding(root, Some(session_id));

    if TRACKED_PIPELINES.lock().contains_key(session_id) {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Pipeline {session_id} is already running"
        )));
    }

    let cancel = CancellationToken::new();
    let (paused, paused_rx) = watch::channel(false);
    let task_context = supervisor_context(root);
    let root = root.to_path_buf();
    let session_id_owned = session_id.to_string();
    let prompt_owned = prompt.to_string();
    let cancel_for_task = cancel.clone();
    let (start_tx, start_rx) = oneshot::channel();

    let handle = supervisor_runtime()?.spawn(async move {
        let _paused_rx = paused_rx;
        if start_rx.await.is_err() {
            finish_pipeline(&session_id_owned);
            return;
        }

        let result = executor::execute_pipeline(
            &task_context,
            &session_id_owned,
            &prompt_owned,
            cancel_for_task,
        )
        .await;

        if let Err(error) = result {
            eprintln!(
                "[pipeline-supervisor] pipeline {} failed: {}",
                session_id_owned, error
            );
            if pipeline_has_non_terminal_nodes(root.as_path(), &session_id_owned) {
                mark_pipeline_stuck(root.as_path(), &session_id_owned);
            }
        }

        finish_pipeline(&session_id_owned);
    });

    TRACKED_PIPELINES.lock().insert(
        session_id.to_string(),
        TrackedPipeline {
            started_at: Instant::now(),
            cancel,
            paused,
            handle,
        },
    );
    RUNNING_PIPELINES.lock().insert(session_id.to_string());
    let _ = start_tx.send(());
    Ok(())
}

pub(super) fn supervisor_cancel(session_id: &str) -> Result<()> {
    let tracked = TRACKED_PIPELINES.lock().remove(session_id).ok_or_else(|| {
        MaenifoldError::InvalidArgument(format!("Pipeline {session_id} isn't running"))
    })?;

    let _ = tracked.paused.send(false);
    tracked.cancel.cancel();
    let join_result = supervisor_runtime()?.block_on(tracked.handle);
    RUNNING_PIPELINES.lock().remove(session_id);
    join_result.map_err(|error| {
        MaenifoldError::Message(format!(
            "ERROR: Pipeline {session_id} task join failed: {error}"
        ))
    })?;
    Ok(())
}

pub(super) fn supervisor_pause(session_id: &str) -> Result<()> {
    let guard = TRACKED_PIPELINES.lock();
    let tracked = guard.get(session_id).ok_or_else(|| {
        MaenifoldError::InvalidArgument(format!("Pipeline {session_id} isn't running"))
    })?;
    tracked.paused.send(true).map_err(|_| {
        MaenifoldError::Message(format!(
            "ERROR: Failed to pause pipeline {session_id}: no active supervisor task"
        ))
    })?;
    Ok(())
}

pub(super) fn supervisor_resume(session_id: &str) -> Result<()> {
    let guard = TRACKED_PIPELINES.lock();
    let tracked = guard.get(session_id).ok_or_else(|| {
        MaenifoldError::InvalidArgument(format!("Pipeline {session_id} isn't running"))
    })?;
    tracked.paused.send(false).map_err(|_| {
        MaenifoldError::Message(format!(
            "ERROR: Failed to resume pipeline {session_id}: no active supervisor task"
        ))
    })?;
    Ok(())
}

pub(super) fn supervisor_status(session_id: &str) -> Option<Instant> {
    TRACKED_PIPELINES
        .lock()
        .get(session_id)
        .map(|tracked| tracked.started_at)
}

pub(super) fn supervisor_is_running(session_id: &str) -> bool {
    RUNNING_PIPELINES.lock().contains(session_id)
}

pub(super) async fn supervisor_wait_if_paused(session_id: &str, cancel: &CancellationToken) {
    let mut paused_rx = {
        let guard = TRACKED_PIPELINES.lock();
        let Some(tracked) = guard.get(session_id) else {
            return;
        };
        tracked.paused.subscribe()
    };

    loop {
        if cancel.is_cancelled() || !*paused_rx.borrow() {
            return;
        }

        tokio::select! {
            _ = cancel.cancelled() => return,
            changed = paused_rx.changed() => {
                if changed.is_err() {
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
pub(super) fn supervisor_clear_all() {
    let tracked = {
        let mut guard = TRACKED_PIPELINES.lock();
        std::mem::take(&mut *guard)
    };
    RUNNING_PIPELINES.lock().clear();

    for (_, tracked) in tracked {
        let _ = tracked.paused.send(false);
        tracked.cancel.cancel();
        if let Ok(runtime) = supervisor_runtime() {
            let _ = runtime.block_on(tracked.handle);
        }
    }
}

#[cfg(test)]
pub(super) fn supervisor_is_paused(session_id: &str) -> Option<bool> {
    TRACKED_PIPELINES
        .lock()
        .get(session_id)
        .map(|tracked| *tracked.paused.borrow())
}

#[cfg(test)]
pub(super) fn supervisor_spawn_test_task(session_id: &str) -> Result<()> {
    if TRACKED_PIPELINES.lock().contains_key(session_id) {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Pipeline {session_id} is already running"
        )));
    }

    let cancel = CancellationToken::new();
    let (paused, _paused_rx) = watch::channel(false);
    let session_id_owned = session_id.to_string();
    let cancel_for_task = cancel.clone();
    let handle = supervisor_runtime()?.spawn(async move {
        cancel_for_task.cancelled().await;
        finish_pipeline(&session_id_owned);
    });

    TRACKED_PIPELINES.lock().insert(
        session_id.to_string(),
        TrackedPipeline {
            started_at: Instant::now(),
            cancel,
            paused,
            handle,
        },
    );
    RUNNING_PIPELINES.lock().insert(session_id.to_string());
    Ok(())
}
