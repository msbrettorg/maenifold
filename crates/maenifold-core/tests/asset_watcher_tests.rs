use maenifold_core::{AssetKind, AssetManager, Config, MaenifoldError};
use notify::{Config as NotifyConfig, Event, EventKind, PollWatcher, RecursiveMode, Watcher};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

const DEFAULT_DEBOUNCE_MS: u64 = 150;

struct ConfigOverrideReset;

impl Drop for ConfigOverrideReset {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

struct WatcherState {
    watcher: PollWatcher,
    control_tx: Sender<WatcherSignal>,
    worker: Option<JoinHandle<()>>,
}

impl WatcherState {
    fn stop(mut self) {
        let _ = self.control_tx.send(WatcherSignal::Stop);
        drop(self.watcher);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

enum WatcherSignal {
    Event(Event),
    Stop,
}

struct TestAssetWatcher {
    assets_path: PathBuf,
    debounce_ms: u64,
    notifications: Arc<AtomicUsize>,
    state: Option<WatcherState>,
    batch_rx: Option<Receiver<Vec<PathBuf>>>,
}

impl TestAssetWatcher {
    fn new(assets_path: PathBuf, debounce_ms: u64) -> Self {
        Self {
            assets_path,
            debounce_ms,
            notifications: Arc::new(AtomicUsize::new(0)),
            state: None,
            batch_rx: None,
        }
    }

    fn start(&mut self) -> notify::Result<String> {
        if self.state.is_some() {
            return Ok("Asset watcher already running".to_string());
        }

        self.notifications.store(0, Ordering::SeqCst);

        let (event_tx, event_rx) = mpsc::channel();
        let (batch_tx, batch_rx) = mpsc::channel();
        let assets_path = self.assets_path.clone();
        let notifications = Arc::clone(&self.notifications);
        let debounce_ms = self.debounce_ms;
        let worker = spawn_worker(
            assets_path.clone(),
            debounce_ms,
            notifications,
            event_rx,
            batch_tx,
        );
        let callback_tx = event_tx.clone();

        let mut watcher = PollWatcher::new(
            move |event: notify::Result<Event>| {
                let Ok(event) = event else {
                    return;
                };
                if !is_relevant_event_kind(&event.kind) {
                    return;
                }
                let _ = callback_tx.send(WatcherSignal::Event(event));
            },
            NotifyConfig::default().with_poll_interval(Duration::from_millis(50)),
        )?;

        if let Err(error) = watcher.watch(&assets_path, RecursiveMode::Recursive) {
            let _ = event_tx.send(WatcherSignal::Stop);
            let _ = worker.join();
            return Err(error);
        }

        self.batch_rx = Some(batch_rx);
        self.state = Some(WatcherState {
            watcher,
            control_tx: event_tx,
            worker: Some(worker),
        });

        Ok(format!(
            "Started watching {} (debounce: {}ms)",
            self.assets_path.display(),
            self.debounce_ms,
        ))
    }

    fn stop(&mut self) -> String {
        if let Some(state) = self.state.take() {
            state.stop();
            self.batch_rx.take();
            return "Stopped asset watcher".to_string();
        }
        "Asset watcher not running".to_string()
    }

    fn notification_count(&self) -> usize {
        self.notifications.load(Ordering::SeqCst)
    }

    fn wait_for_notifications(&self, expected: usize, timeout: Duration) {
        wait_for(
            &format!("asset watcher notifications >= {expected}"),
            timeout,
            || self.notification_count() >= expected,
        );
    }

    fn drain_batches(&mut self) -> Vec<Vec<PathBuf>> {
        let mut batches = Vec::new();
        let Some(receiver) = self.batch_rx.as_ref() else {
            return batches;
        };

        while let Ok(batch) = receiver.try_recv() {
            batches.push(batch);
        }

        batches
    }

    fn wait_for_batch_with_path(&mut self, target: &Path, timeout: Duration) -> Vec<PathBuf> {
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            assert!(
                !remaining.is_zero(),
                "timed out waiting for batch containing {}",
                target.display()
            );

            let receiver = self
                .batch_rx
                .as_ref()
                .expect("batch receiver should exist while watcher is running");
            match receiver.recv_timeout(remaining.min(Duration::from_millis(25))) {
                Ok(batch) if batch.iter().any(|path| path == target) => return batch,
                Ok(_) => continue,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => {
                    panic!("asset watcher batch channel disconnected")
                }
            }
        }
    }

    fn start_and_settle(&mut self) -> notify::Result<String> {
        let result = self.start()?;
        thread::sleep(Duration::from_millis(200));
        Ok(result)
    }
}

impl Drop for TestAssetWatcher {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

struct AssetWatcherHarness {
    _tmp: TempDir,
    _reset: ConfigOverrideReset,
    config: Config,
    assets_path: PathBuf,
    workflows_path: PathBuf,
    watcher: TestAssetWatcher,
}

impl AssetWatcherHarness {
    fn new() -> Self {
        let tmp = TempDir::new().expect("tmp");
        Config::override_root(tmp.path());
        Config::set_database_path(tmp.path().join("memory.db"));
        let config = Config::current();
        config.ensure_directories().expect("dirs");

        let assets_path = config.assets_path.clone();
        let workflows_path = assets_path.join("workflows");
        fs::create_dir_all(&workflows_path).expect("create workflows directory");

        Self {
            _tmp: tmp,
            _reset: ConfigOverrideReset,
            config,
            assets_path: assets_path.clone(),
            workflows_path,
            watcher: TestAssetWatcher::new(assets_path, DEFAULT_DEBOUNCE_MS),
        }
    }

    fn asset_manager(&self) -> AssetManager {
        AssetManager::new(&self.config)
    }
}

fn spawn_worker(
    assets_path: PathBuf,
    debounce_ms: u64,
    notifications: Arc<AtomicUsize>,
    event_rx: Receiver<WatcherSignal>,
    batch_tx: Sender<Vec<PathBuf>>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let debounce_window = Duration::from_millis(debounce_ms.max(1));
        let mut pending_paths = HashSet::new();

        loop {
            match event_rx.recv() {
                Ok(WatcherSignal::Event(event)) => {
                    collect_pending_paths(&assets_path, &event, &mut pending_paths);
                }
                Ok(WatcherSignal::Stop) | Err(_) => {
                    flush_pending_paths(&notifications, &batch_tx, &mut pending_paths);
                    break;
                }
            }

            loop {
                match event_rx.recv_timeout(debounce_window) {
                    Ok(WatcherSignal::Event(event)) => {
                        collect_pending_paths(&assets_path, &event, &mut pending_paths);
                    }
                    Ok(WatcherSignal::Stop) => {
                        flush_pending_paths(&notifications, &batch_tx, &mut pending_paths);
                        return;
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        flush_pending_paths(&notifications, &batch_tx, &mut pending_paths);
                        break;
                    }
                    Err(RecvTimeoutError::Disconnected) => {
                        flush_pending_paths(&notifications, &batch_tx, &mut pending_paths);
                        return;
                    }
                }
            }
        }
    })
}

fn collect_pending_paths(assets_path: &Path, event: &Event, pending_paths: &mut HashSet<PathBuf>) {
    for path in &event.paths {
        if is_watchable_asset_json_path(assets_path, path) {
            pending_paths.insert(path.clone());
        }
    }
}

fn flush_pending_paths(
    notifications: &AtomicUsize,
    batch_tx: &Sender<Vec<PathBuf>>,
    pending_paths: &mut HashSet<PathBuf>,
) {
    if pending_paths.is_empty() {
        return;
    }

    let mut batch = pending_paths.drain().collect::<Vec<_>>();
    batch.sort();
    notifications.fetch_add(1, Ordering::SeqCst);
    let _ = batch_tx.send(batch);
}

fn is_relevant_event_kind(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Any | EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

fn is_watchable_asset_json_path(assets_path: &Path, path: &Path) -> bool {
    path.starts_with(assets_path)
        && path.extension().and_then(|ext| ext.to_str()) == Some("json")
        && !is_hidden_relative_path(assets_path, path)
}

fn is_hidden_relative_path(assets_path: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(assets_path) else {
        return true;
    };

    relative.components().any(|component| {
        let text = component.as_os_str().to_string_lossy();
        text.starts_with('.') && text.len() > 1
    })
}

fn wait_for(description: &str, timeout: Duration, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if condition() {
            return;
        }
        thread::sleep(Duration::from_millis(25));
    }

    panic!("timed out waiting for {description}");
}

// T-0.1: RTM NFR-3 — ports AssetWatcherToolsTests.cs StartAssetWatcher_CreatesWatcherSuccessfully.
#[test]
fn start_asset_watcher_creates_watcher_successfully() {
    let _guard = TEST_MUTEX.lock();
    let mut harness = AssetWatcherHarness::new();

    let result = harness.watcher.start().expect("start asset watcher");

    assert!(result.contains("Started watching"));
    assert!(result.contains(&harness.assets_path.display().to_string()));
    assert!(
        result.contains("150ms"),
        "default debounce should remain visible in the start message: {result}"
    );
}

// T-0.1: RTM NFR-3 — ports AssetWatcherToolsTests.cs StartAssetWatcher_WhenAlreadyRunning_ReturnsAlreadyRunningMessage.
#[test]
fn start_asset_watcher_when_already_running_returns_already_running_message() {
    let _guard = TEST_MUTEX.lock();
    let mut harness = AssetWatcherHarness::new();

    harness.watcher.start().expect("first start");
    let result = harness.watcher.start().expect("second start");

    assert_eq!(result, "Asset watcher already running");
}

// T-0.1: RTM NFR-3 — ports AssetWatcherToolsTests.cs StopAssetWatcher_WhenRunning_StopsSuccessfully.
#[test]
fn stop_asset_watcher_when_running_stops_successfully() {
    let _guard = TEST_MUTEX.lock();
    let mut harness = AssetWatcherHarness::new();

    harness.watcher.start().expect("start");
    let result = harness.watcher.stop();

    assert_eq!(result, "Stopped asset watcher");
}

// T-0.1: RTM NFR-3 — ports AssetWatcherToolsTests.cs StopAssetWatcher_WhenNotRunning_ReturnsNotRunningMessage.
#[test]
fn stop_asset_watcher_when_not_running_returns_not_running_message() {
    let _guard = TEST_MUTEX.lock();
    let mut harness = AssetWatcherHarness::new();

    let result = harness.watcher.stop();

    assert_eq!(result, "Asset watcher not running");
}

// T-0.1: RTM NFR-3 — ports AssetWatcherToolsTests.cs FileCreated_TriggersDebounceAndNotification.
#[test]
fn file_created_triggers_debounce_and_notification() {
    let _guard = TEST_MUTEX.lock();
    let mut harness = AssetWatcherHarness::new();
    harness.watcher.start_and_settle().expect("start");

    let test_file = harness.workflows_path.join("test-create.json");
    let test_content = r#"{"name":"test-workflow","version":"1.0.0"}"#;
    fs::write(&test_file, test_content).expect("create workflow asset");

    let batch = harness
        .watcher
        .wait_for_batch_with_path(&test_file, Duration::from_secs(5));

    assert!(test_file.exists(), "file should exist after creation");
    assert!(
        batch.iter().any(|path| path == &test_file),
        "debounced batch should include the created file: {batch:?}"
    );
    let asset = harness
        .asset_manager()
        .read_asset(AssetKind::Workflow, "test-create")
        .expect("created asset should be readable");
    assert_eq!(asset["name"], "test-workflow");
    assert_eq!(asset["version"], "1.0.0");
}

// T-0.1: RTM NFR-3 — ports AssetWatcherToolsTests.cs FileChanged_TriggersDebounceAndNotification.
#[test]
fn file_changed_triggers_debounce_and_notification() {
    let _guard = TEST_MUTEX.lock();
    let mut harness = AssetWatcherHarness::new();
    harness.watcher.start_and_settle().expect("start");

    let test_file = harness.workflows_path.join("test-modify.json");
    fs::write(&test_file, r#"{"version":"1.0.0"}"#).expect("write initial asset");
    harness
        .watcher
        .wait_for_notifications(1, Duration::from_secs(5));

    fs::write(&test_file, r#"{"version":"2.0.0"}"#).expect("write updated asset");
    let batch = harness
        .watcher
        .wait_for_batch_with_path(&test_file, Duration::from_secs(5));

    let content = fs::read_to_string(&test_file).expect("read updated asset");
    assert!(
        content.contains("2.0.0"),
        "file should contain updated content"
    );
    assert!(
        batch.iter().any(|path| path == &test_file),
        "debounced batch should include the changed file: {batch:?}"
    );
    let asset = harness
        .asset_manager()
        .read_asset(AssetKind::Workflow, "test-modify")
        .expect("updated asset should be readable");
    assert_eq!(asset["version"], "2.0.0");
}

// T-0.1: RTM NFR-3 — ports AssetWatcherToolsTests.cs FileDeleted_TriggersDebounceAndNotification.
#[test]
fn file_deleted_triggers_debounce_and_notification() {
    let _guard = TEST_MUTEX.lock();
    let mut harness = AssetWatcherHarness::new();
    harness.watcher.start_and_settle().expect("start");

    let test_file = harness.workflows_path.join("test-delete.json");
    fs::write(&test_file, r#"{"name":"delete-me"}"#).expect("write asset");
    harness
        .watcher
        .wait_for_notifications(1, Duration::from_secs(5));

    fs::remove_file(&test_file).expect("delete asset");
    let batch = harness
        .watcher
        .wait_for_batch_with_path(&test_file, Duration::from_secs(5));

    assert!(!test_file.exists(), "file should not exist after deletion");
    assert!(
        batch.iter().any(|path| path == &test_file),
        "debounced batch should include the deleted file path: {batch:?}"
    );
    let error = harness
        .asset_manager()
        .read_asset(AssetKind::Workflow, "test-delete")
        .expect_err("deleted asset should no longer be readable");
    assert!(
        matches!(&error, MaenifoldError::AssetNotFound(kind, id) if kind == "workflow" && id == "test-delete"),
        "expected missing asset error after deletion, got: {error:?}"
    );
}

// T-0.1: RTM NFR-3 — ports AssetWatcherToolsTests.cs FileRenamed_TriggersDebounceAndNotification.
#[test]
fn file_renamed_triggers_debounce_and_notification() {
    let _guard = TEST_MUTEX.lock();
    let mut harness = AssetWatcherHarness::new();
    harness.watcher.start_and_settle().expect("start");

    let old_file = harness.workflows_path.join("test-old-name.json");
    let new_file = harness.workflows_path.join("test-new-name.json");
    fs::write(&old_file, r#"{"name":"rename-test"}"#).expect("write old asset");
    harness
        .watcher
        .wait_for_notifications(1, Duration::from_secs(5));

    fs::rename(&old_file, &new_file).expect("rename asset");
    harness
        .watcher
        .wait_for_notifications(2, Duration::from_secs(5));
    let batches = harness.watcher.drain_batches();

    assert!(new_file.exists(), "renamed file should exist at new path");
    assert!(!old_file.exists(), "old path should not exist after rename");
    assert!(
        batches
            .iter()
            .flatten()
            .any(|path| path == &old_file || path == &new_file),
        "rename should surface old or new file path in a debounced batch: {batches:?}"
    );
    assert!(
        harness
            .asset_manager()
            .read_asset(AssetKind::Workflow, "test-new-name")
            .is_ok(),
        "renamed asset should be readable by new filename"
    );
}

// T-0.1: RTM NFR-3 — ports AssetWatcherToolsTests.cs RapidFileChanges_AreDebouncedIntoSingleNotification.
#[test]
fn rapid_file_changes_are_debounced_into_single_notification() {
    let _guard = TEST_MUTEX.lock();
    let mut harness = AssetWatcherHarness::new();
    harness.watcher.start_and_settle().expect("start");

    let test_file = harness.workflows_path.join("test-rapid.json");
    for version in 0..5 {
        fs::write(&test_file, format!(r#"{{"version":"{version}"}}"#)).expect("write rapid asset");
        if version < 4 {
            thread::sleep(Duration::from_millis(30));
        }
    }

    harness
        .watcher
        .wait_for_notifications(1, Duration::from_secs(5));
    thread::sleep(Duration::from_millis(300));

    let content = fs::read_to_string(&test_file).expect("read final asset");
    assert!(content.contains('4'), "file should contain final version");
    assert_eq!(
        harness.watcher.notification_count(),
        1,
        "rapid changes inside one debounce window should collapse into a single notification"
    );
}

// T-0.1: RTM NFR-3 — ports AssetWatcherToolsTests.cs DebounceLatency_IsUnder500ms.
#[test]
fn debounce_latency_is_under_500ms() {
    let _guard = TEST_MUTEX.lock();
    let mut harness = AssetWatcherHarness::new();
    harness.watcher.start_and_settle().expect("start");

    let test_file = harness.workflows_path.join("test-latency.json");
    let started = Instant::now();
    fs::write(&test_file, r#"{"name":"latency-test"}"#).expect("write latency asset");
    harness
        .watcher
        .wait_for_notifications(1, Duration::from_secs(5));
    let elapsed = started.elapsed();

    assert!(
        elapsed < Duration::from_millis(500),
        "debounce latency was {}ms, expected < 500ms",
        elapsed.as_millis()
    );
    assert!(test_file.exists(), "file should exist after latency test");
}
