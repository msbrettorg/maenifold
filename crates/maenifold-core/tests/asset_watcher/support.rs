#![allow(dead_code, unused_imports)]

// T-12.7: RTM NFR-1 - shared asset watcher harness extracted so themed test files stay focused.
#[allow(clippy::duplicate_mod)]
#[path = "runtime.rs"]
mod asset_watcher_runtime;

use asset_watcher_runtime::{is_relevant_event_kind, spawn_worker, wait_for};
use maenifold_core::{AssetManager, Config};
use notify::{Config as NotifyConfig, Event, PollWatcher, RecursiveMode, Watcher};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tempfile::TempDir;

pub(crate) static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub(crate) const DEFAULT_DEBOUNCE_MS: u64 = 150;

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

pub(crate) struct TestAssetWatcher {
    assets_path: PathBuf,
    debounce_ms: u64,
    notifications: Arc<AtomicUsize>,
    state: Option<WatcherState>,
    batch_rx: Option<Receiver<Vec<PathBuf>>>,
}

impl TestAssetWatcher {
    pub(crate) fn new(assets_path: PathBuf, debounce_ms: u64) -> Self {
        Self {
            assets_path,
            debounce_ms,
            notifications: Arc::new(AtomicUsize::new(0)),
            state: None,
            batch_rx: None,
        }
    }

    pub(crate) fn start(&mut self) -> notify::Result<String> {
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

    pub(crate) fn stop(&mut self) -> String {
        if let Some(state) = self.state.take() {
            state.stop();
            self.batch_rx.take();
            return "Stopped asset watcher".to_string();
        }
        "Asset watcher not running".to_string()
    }

    pub(crate) fn notification_count(&self) -> usize {
        self.notifications.load(Ordering::SeqCst)
    }

    pub(crate) fn wait_for_notifications(&self, expected: usize, timeout: Duration) {
        wait_for(
            &format!("asset watcher notifications >= {expected}"),
            timeout,
            || self.notification_count() >= expected,
        );
    }

    pub(crate) fn drain_batches(&mut self) -> Vec<Vec<PathBuf>> {
        let mut batches = Vec::new();
        let Some(receiver) = self.batch_rx.as_ref() else {
            return batches;
        };

        while let Ok(batch) = receiver.try_recv() {
            batches.push(batch);
        }

        batches
    }

    pub(crate) fn wait_for_batch_with_path(
        &mut self,
        target: &Path,
        timeout: Duration,
    ) -> Vec<PathBuf> {
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

    pub(crate) fn start_and_settle(&mut self) -> notify::Result<String> {
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

pub(crate) struct AssetWatcherHarness {
    _tmp: TempDir,
    _reset: ConfigOverrideReset,
    config: Config,
    pub(crate) workflows_path: PathBuf,
    pub(crate) watcher: TestAssetWatcher,
}

impl AssetWatcherHarness {
    pub(crate) fn new() -> Self {
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
            workflows_path,
            watcher: TestAssetWatcher::new(assets_path, DEFAULT_DEBOUNCE_MS),
        }
    }

    pub(crate) fn asset_manager(&self) -> AssetManager {
        AssetManager::new(&self.config)
    }
}
