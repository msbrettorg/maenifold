use std::{
    collections::HashMap,
    sync::LazyLock,
    time::{Duration, Instant},
};

use parking_lot::Mutex;
use tokio::runtime::{Builder, Handle, Runtime};

use crate::mcp::resources::{
    notify_resource_updated,
    pipeline::{PIPELINE_SESSION_PREFIX, PIPELINE_SESSIONS_URI},
};

const PIPELINE_NOTIFICATION_THROTTLE_WINDOW: Duration = Duration::from_millis(500);

static NOTIFICATION_RUNTIME: LazyLock<std::result::Result<Runtime, String>> = LazyLock::new(|| {
    Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("Failed to build pipeline notification runtime: {error}"))
});

static PIPELINE_RESOURCE_STATES: LazyLock<Mutex<HashMap<String, PipelineResourceEmitState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

struct PipelineResourceEmitState {
    last_emit_at: Instant,
    trailing_scheduled: bool,
}

enum PipelineResourceEmitPlan {
    Immediate,
    ScheduleTrailing(Duration),
    Suppressed,
}

pub(super) fn emit_pipeline_resource_updated(session_id: &str) {
    let session_id = session_id.to_string();
    run_notification(async move {
        match plan_session_update(&session_id) {
            PipelineResourceEmitPlan::Immediate => {
                emit_pipeline_resource_update_pair(&session_id).await;
            }
            PipelineResourceEmitPlan::ScheduleTrailing(delay) => {
                spawn_trailing_session_update(session_id, delay);
            }
            PipelineResourceEmitPlan::Suppressed => {}
        }
    });
}

pub(super) fn emit_pipeline_sessions_resource_updated() {
    run_notification(async {
        notify_resource_updated(PIPELINE_SESSIONS_URI).await;
    });
}

pub(super) fn clear_pipeline_resource_state(session_id: &str) {
    PIPELINE_RESOURCE_STATES.lock().remove(session_id);
}

fn run_notification(future: impl std::future::Future<Output = ()> + Send + 'static) {
    if let Ok(handle) = Handle::try_current() {
        handle.spawn(future);
        return;
    }

    if let Ok(runtime) = NOTIFICATION_RUNTIME.as_ref() {
        runtime.block_on(future);
    }
}

fn plan_session_update(session_id: &str) -> PipelineResourceEmitPlan {
    let now = Instant::now();
    let mut states = PIPELINE_RESOURCE_STATES.lock();

    let Some(state) = states.get_mut(session_id) else {
        states.insert(
            session_id.to_string(),
            PipelineResourceEmitState {
                last_emit_at: now,
                trailing_scheduled: false,
            },
        );
        return PipelineResourceEmitPlan::Immediate;
    };

    if state.trailing_scheduled {
        return PipelineResourceEmitPlan::Suppressed;
    }

    let elapsed = now.saturating_duration_since(state.last_emit_at);
    if elapsed >= PIPELINE_NOTIFICATION_THROTTLE_WINDOW {
        state.last_emit_at = now;
        return PipelineResourceEmitPlan::Immediate;
    }

    state.trailing_scheduled = true;
    PipelineResourceEmitPlan::ScheduleTrailing(PIPELINE_NOTIFICATION_THROTTLE_WINDOW - elapsed)
}

fn mark_trailing_session_update_emitted(session_id: &str) {
    let mut states = PIPELINE_RESOURCE_STATES.lock();
    let state = states
        .entry(session_id.to_string())
        .or_insert_with(|| PipelineResourceEmitState {
            last_emit_at: Instant::now(),
            trailing_scheduled: false,
        });
    state.last_emit_at = Instant::now();
    state.trailing_scheduled = false;
}

fn spawn_trailing_session_update(session_id: String, delay: Duration) {
    let thread_name = format!("pipeline-resource-update-{session_id}");
    let fallback_id = session_id.clone();
    if let Err(error) = std::thread::Builder::new()
        .name(thread_name)
        .spawn(move || {
            std::thread::sleep(delay);
            run_notification(async move {
                mark_trailing_session_update_emitted(&session_id);
                emit_pipeline_resource_update_pair(&session_id).await;
            });
        })
    {
        eprintln!("Warning: failed to schedule pipeline resource update: {error}");
        mark_trailing_session_update_emitted(&fallback_id);
        run_notification(async move {
            emit_pipeline_resource_update_pair(&fallback_id).await;
        });
    }
}

async fn emit_pipeline_resource_update_pair(session_id: &str) {
    notify_resource_updated(PIPELINE_SESSIONS_URI).await;
    notify_resource_updated(&format!("{PIPELINE_SESSION_PREFIX}{session_id}")).await;
}
