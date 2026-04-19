use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use notify::{Config as NotifyConfig, Event, EventKind, RecursiveMode, Watcher};
use rmcp::{Peer, RoleServer};
use std::sync::mpsc::{self, Sender};
use std::thread::JoinHandle;
use std::time::Duration;

use super::subscription_worker::{AssetWatcherSignal, spawn_asset_watcher_worker};

pub struct AssetWatcherState {
    watcher: notify::RecommendedWatcher,
    control_tx: Sender<AssetWatcherSignal>,
    worker: Option<JoinHandle<()>>,
}

impl AssetWatcherState {
    pub fn stop(mut self) {
        let _ = self.control_tx.send(AssetWatcherSignal::Stop);
        drop(self.watcher);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub fn start_asset_watcher(config: &Config, peer: Peer<RoleServer>) -> Result<AssetWatcherState> {
    let assets_path = config.assets_path.clone();
    let runtime = tokio::runtime::Handle::current();
    let (event_tx, event_rx) = mpsc::channel();
    let worker = spawn_asset_watcher_worker(assets_path.clone(), 150_u64, peer, runtime, event_rx);
    let callback_tx = event_tx.clone();

    let mut watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
        let Ok(event) = event else { return };
        if !matches!(
            event.kind,
            EventKind::Any | EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
        ) {
            return;
        }
        let _ = callback_tx.send(AssetWatcherSignal::Event(event));
    })
    .map_err(map_notify_error)?;

    let _ = watcher.configure(
        NotifyConfig::default()
            .with_poll_interval(Duration::from_millis(50))
            .with_compare_contents(true),
    );

    if let Err(error) = watcher.watch(&assets_path, RecursiveMode::Recursive) {
        let _ = event_tx.send(AssetWatcherSignal::Stop);
        let _ = worker.join();
        return Err(map_notify_error(error));
    }

    Ok(AssetWatcherState {
        watcher,
        control_tx: event_tx,
        worker: Some(worker),
    })
}

fn map_notify_error(error: notify::Error) -> MaenifoldError {
    MaenifoldError::Message(format!("Asset watcher failed: {error}"))
}
