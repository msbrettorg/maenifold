use crate::assets::is_watchable_asset_json_path;
use rmcp::{Peer, RoleServer};
use std::collections::HashSet;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use super::subscription::{asset_uri_from_path, notify_resource_updated};

pub(super) enum AssetWatcherSignal {
    Event(notify::Event),
    Stop,
}

pub(super) fn spawn_asset_watcher_worker(
    assets_path: std::path::PathBuf,
    debounce_ms: u64,
    peer: Peer<RoleServer>,
    runtime: tokio::runtime::Handle,
    event_rx: Receiver<AssetWatcherSignal>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let debounce_window = Duration::from_millis(debounce_ms.max(1));
        let flush_grace_window = debounce_window.min(Duration::from_millis(50));
        let mut pending_paths = HashSet::new();

        loop {
            let pending = &mut pending_paths;
            let watchable =
                |path: &std::path::Path| is_watchable_asset_json_path(&assets_path, path);

            match event_rx.recv() {
                Ok(AssetWatcherSignal::Event(event)) => {
                    event.paths.iter().filter(|p| watchable(p)).for_each(|p| {
                        pending.insert(p.clone());
                    })
                }
                Ok(AssetWatcherSignal::Stop) | Err(_) => {
                    return flush(&assets_path, &runtime, &peer, pending);
                }
            }

            loop {
                match event_rx.recv_timeout(debounce_window) {
                    Ok(AssetWatcherSignal::Event(event)) => {
                        event.paths.iter().filter(|p| watchable(p)).for_each(|p| {
                            pending.insert(p.clone());
                        })
                    }
                    Ok(AssetWatcherSignal::Stop) => {
                        return flush(&assets_path, &runtime, &peer, pending);
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        match event_rx.recv_timeout(flush_grace_window) {
                            Ok(AssetWatcherSignal::Event(event)) => {
                                event.paths.iter().filter(|p| watchable(p)).for_each(|p| {
                                    pending.insert(p.clone());
                                })
                            }
                            Ok(AssetWatcherSignal::Stop) | Err(RecvTimeoutError::Disconnected) => {
                                return flush(&assets_path, &runtime, &peer, pending);
                            }
                            Err(RecvTimeoutError::Timeout) => {
                                flush(&assets_path, &runtime, &peer, pending);
                                break;
                            }
                        }
                    }
                    Err(RecvTimeoutError::Disconnected) => {
                        return flush(&assets_path, &runtime, &peer, pending);
                    }
                }
            }
        }
    })
}

fn flush(
    assets_path: &std::path::Path,
    runtime: &tokio::runtime::Handle,
    peer: &Peer<RoleServer>,
    pending_paths: &mut HashSet<std::path::PathBuf>,
) {
    if pending_paths.is_empty() {
        return;
    }
    let uris = pending_paths
        .iter()
        .filter_map(|path| asset_uri_from_path(assets_path, path))
        .collect::<HashSet<_>>();
    pending_paths.clear();
    let notification_peer = peer.clone();
    runtime.block_on(async move {
        let _ = notification_peer.notify_resource_list_changed().await;
        for uri in uris {
            notify_resource_updated(&uri).await;
        }
    });
}
