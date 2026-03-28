// T-12.0: RTM NFR-1 / FR-11 — resource enumeration, reads, and asset watcher
// notification handling now live in mcp/resources.rs behind the stable crate::mcp facade.
use crate::assets::{AssetKind, AssetManager, is_watchable_asset_json_path};
use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use notify::{Config as NotifyConfig, Event, EventKind, RecursiveMode, Watcher};
use rmcp::model::{
    AnnotateAble, ListResourcesResult, RawResource, ReadResourceResult, Resource, ResourceContents,
};
use rmcp::{Peer, RoleServer};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

// T-12.8: RTM FR-11 — extracted MCP resources/list, resources/read, and asset watcher logic
// from mcp.rs into a dedicated module matching the C# AssetResources/AssetWatcher split.
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

enum AssetWatcherSignal {
    Event(Event),
    Stop,
}

// T-12.8: RTM FR-11 / RTM-069 — resources/list assembly now lives in mcp/resources.rs.
pub fn handle_list_resources() -> Result<ListResourcesResult> {
    list_asset_resources(current_asset_manager()?).map(|resources| ListResourcesResult {
        resources,
        next_cursor: None,
        meta: None,
    })
}

// T-12.0: RTM FR-11 / RTM-070 — keep resources/read explicit so the split module
// remains straightforward to parse and preserves the current JSON response shape.
pub fn handle_read_resource(uri: &str) -> Result<ReadResourceResult> {
    let value = current_asset_manager()?.read_resource(uri)?;
    let text = serde_json::to_string_pretty(&value)?;
    Ok(ReadResourceResult::new(vec![
        ResourceContents::text(text, uri.to_string()).with_mime_type("application/json"),
    ]))
}

// T-27.7: RTM FR-37 / RTM-236 — resource handlers rebuild the current AssetManager on
// demand without re-running runtime initialization, which already happens during server
// startup.
pub(crate) fn current_asset_manager() -> Result<AssetManager> {
    let config = Config::current();
    Ok(AssetManager::new(&config))
}

pub fn start_asset_watcher(config: &Config, peer: Peer<RoleServer>) -> Result<AssetWatcherState> {
    let assets_path = config.assets_path.clone();
    let debounce_ms = 150_u64;
    let runtime = tokio::runtime::Handle::current();
    let (event_tx, event_rx) = mpsc::channel();
    let worker =
        spawn_asset_watcher_worker(assets_path.clone(), debounce_ms, peer, runtime, event_rx);
    let callback_tx = event_tx.clone();

    let mut watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
        let Ok(event) = event else {
            return;
        };
        if !is_relevant_asset_event_kind(&event.kind) {
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

pub(crate) fn list_asset_resources(asset_manager: AssetManager) -> Result<Vec<Resource>> {
    let mut resources = Vec::new();
    for kind in AssetKind::all() {
        let metadata = asset_manager.list_assets(Some(kind))?;
        let Value::Array(items) = metadata else {
            continue;
        };
        for item in items {
            if let Some(resource) = resource_from_metadata(kind, item) {
                resources.push(resource);
            }
        }
    }
    Ok(resources)
}

pub fn map_resource_error(err: MaenifoldError) -> rmcp::ErrorData {
    match err {
        // T-10.2: RTM FR-11 / RTM-070 — missing assets map to MCP resource-not-found.
        MaenifoldError::AssetNotFound(kind, id) => {
            rmcp::ErrorData::resource_not_found(format!("Asset not found: {kind}/{id}"), None)
        }
        // T-10.2: RTM FR-11 / RTM-069 / RTM-070 — malformed URIs stay invalid-params errors.
        MaenifoldError::UnknownTool(_)
        | MaenifoldError::InvalidPayload(_)
        | MaenifoldError::InvalidArgument(_)
        | MaenifoldError::InvalidResourceUri(_)
        | MaenifoldError::PathTraversal(_)
        | MaenifoldError::MissingProperty(_) => {
            rmcp::ErrorData::invalid_params(err.to_string(), None)
        }
        _ => rmcp::ErrorData::internal_error(err.to_string(), None),
    }
}

fn spawn_asset_watcher_worker(
    assets_path: std::path::PathBuf,
    debounce_ms: u64,
    peer: Peer<RoleServer>,
    runtime: tokio::runtime::Handle,
    event_rx: Receiver<AssetWatcherSignal>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let debounce_window = Duration::from_millis(debounce_ms.max(1));
        let flush_grace_window = debounce_grace_window(debounce_window);
        let mut pending_paths = HashSet::new();

        loop {
            match event_rx.recv() {
                Ok(AssetWatcherSignal::Event(event)) => {
                    collect_pending_asset_paths(&assets_path, &event, &mut pending_paths);
                }
                Ok(AssetWatcherSignal::Stop) | Err(_) => {
                    flush_asset_notifications(&runtime, &peer, &mut pending_paths);
                    break;
                }
            }

            loop {
                match event_rx.recv_timeout(debounce_window) {
                    Ok(AssetWatcherSignal::Event(event)) => {
                        collect_pending_asset_paths(&assets_path, &event, &mut pending_paths);
                    }
                    Ok(AssetWatcherSignal::Stop) => {
                        flush_asset_notifications(&runtime, &peer, &mut pending_paths);
                        return;
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        match event_rx.recv_timeout(flush_grace_window) {
                            Ok(AssetWatcherSignal::Event(event)) => {
                                collect_pending_asset_paths(
                                    &assets_path,
                                    &event,
                                    &mut pending_paths,
                                );
                            }
                            Ok(AssetWatcherSignal::Stop) => {
                                flush_asset_notifications(&runtime, &peer, &mut pending_paths);
                                return;
                            }
                            Err(RecvTimeoutError::Timeout) => {
                                flush_asset_notifications(&runtime, &peer, &mut pending_paths);
                                break;
                            }
                            Err(RecvTimeoutError::Disconnected) => {
                                flush_asset_notifications(&runtime, &peer, &mut pending_paths);
                                return;
                            }
                        }
                    }
                    Err(RecvTimeoutError::Disconnected) => {
                        flush_asset_notifications(&runtime, &peer, &mut pending_paths);
                        return;
                    }
                }
            }
        }
    })
}

fn debounce_grace_window(debounce_window: Duration) -> Duration {
    debounce_window.min(Duration::from_millis(50))
}

fn collect_pending_asset_paths(
    assets_path: &std::path::Path,
    event: &Event,
    pending_paths: &mut HashSet<std::path::PathBuf>,
) {
    for path in &event.paths {
        if is_watchable_asset_json_path(assets_path, path) {
            pending_paths.insert(path.clone());
        }
    }
}

fn flush_asset_notifications(
    runtime: &tokio::runtime::Handle,
    peer: &Peer<RoleServer>,
    pending_paths: &mut HashSet<std::path::PathBuf>,
) {
    if pending_paths.is_empty() {
        return;
    }

    pending_paths.clear();
    let notification_peer = peer.clone();
    runtime.block_on(async move {
        let _ = notification_peer.notify_resource_list_changed().await;
    });
}

fn is_relevant_asset_event_kind(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Any | EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

fn resource_from_metadata(kind: AssetKind, metadata: Value) -> Option<Resource> {
    let object = metadata.as_object()?;
    let id = object.get("id")?.as_str()?;
    let name = object
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(id)
        .to_string();
    let uri = asset_uri(kind, id);
    let mut resource = RawResource::new(uri, name)
        .with_mime_type("application/json")
        .optional_annotate(None);
    if let Some(title) = object.get("name").and_then(Value::as_str) {
        resource.title = Some(title.to_string());
    }
    if let Some(description) = object.get("description").and_then(Value::as_str) {
        resource.description = Some(description.to_string());
    }
    Some(resource)
}

fn asset_uri(kind: AssetKind, id: &str) -> String {
    format!("asset://{}/{id}", kind.dir_name())
}

fn map_notify_error(error: notify::Error) -> MaenifoldError {
    MaenifoldError::Message(format!("Asset watcher failed: {error}"))
}

#[cfg(test)]
mod resources_tests;
