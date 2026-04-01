use crate::config::Config;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Instant;
use tokio::runtime::{Builder, Runtime};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

#[cfg(test)]
pub(super) use super::supervisor_detection::{mark_pipeline_stuck, supervisor_detect_orphans};
pub(super) use super::supervisor_lifecycle::{
    supervisor_cancel, supervisor_is_running, supervisor_pause, supervisor_resume, supervisor_run,
    supervisor_status, supervisor_wait_if_paused,
};
#[cfg(test)]
pub(super) use super::supervisor_lifecycle::{
    supervisor_clear_all, supervisor_is_paused, supervisor_spawn_test_task,
};

pub(crate) static SUPERVISOR_RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("build pipeline supervisor runtime")
});
pub(super) static RUNNING_PIPELINES: Lazy<Mutex<HashSet<String>>> =
    Lazy::new(|| Mutex::new(HashSet::new()));
pub(super) static TRACKED_PIPELINES: Lazy<Mutex<HashMap<String, TrackedPipeline>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub(super) struct TrackedPipeline {
    pub(super) started_at: Instant,
    pub(super) cancel: CancellationToken,
    pub(super) paused: Arc<AtomicBool>,
    pub(super) handle: JoinHandle<()>,
}

pub(super) fn finish_pipeline(session_id: &str) {
    TRACKED_PIPELINES.lock().remove(session_id);
    RUNNING_PIPELINES.lock().remove(session_id);
}

pub(super) fn supervisor_context(root: &Path) -> ToolContext {
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
