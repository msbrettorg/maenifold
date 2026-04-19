use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use axum::http::request::Parts as HttpRequestParts;
use notify::{Config as NotifyConfig, Event, EventKind, RecursiveMode, Watcher};
use rmcp::model::ResourceUpdatedNotificationParam;
use rmcp::service::RequestContext;
use rmcp::{Peer, RoleServer};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Sender};
use std::sync::{LazyLock, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use super::subscription_worker::{spawn_asset_watcher_worker, AssetWatcherSignal};

pub const MAX_SUBSCRIPTIONS_PER_SESSION: usize = 100;
const SESSION_CAP_ERROR: &str = "subscription limit exceeded (max 100 per session)";
const HEADER_SESSION_ID: &str = "mcp-session-id";

#[derive(Clone)]
struct SessionSubscription {
    peer: Peer<RoleServer>,
    uris: HashSet<String>,
}

#[derive(Default)]
struct SubscriptionRegistry {
    sessions: HashMap<String, SessionSubscription>,
}

static SUBSCRIPTIONS: LazyLock<Mutex<SubscriptionRegistry>> =
    LazyLock::new(|| Mutex::new(SubscriptionRegistry::default()));

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
        prune_closed_subscriptions();
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

pub fn subscribe(
    uri: &str,
    context: &RequestContext<RoleServer>,
) -> std::result::Result<(), rmcp::ErrorData> {
    let session_key = session_key(context);
    let mut guard = SUBSCRIPTIONS.lock().expect("subscription registry lock");
    prune_closed_subscriptions_locked(&mut guard);

    let entry = guard
        .sessions
        .entry(session_key)
        .or_insert_with(|| SessionSubscription {
            peer: context.peer.clone(),
            uris: HashSet::new(),
        });

    entry.peer = context.peer.clone();

    if entry.uris.contains(uri) {
        return Ok(());
    }
    if entry.uris.len() >= MAX_SUBSCRIPTIONS_PER_SESSION {
        return Err(rmcp::ErrorData::invalid_params(SESSION_CAP_ERROR, None));
    }

    entry.uris.insert(uri.to_string());
    Ok(())
}

pub fn unsubscribe(uri: &str, context: &RequestContext<RoleServer>) {
    let session_key = session_key(context);
    let mut guard = SUBSCRIPTIONS.lock().expect("subscription registry lock");
    prune_closed_subscriptions_locked(&mut guard);

    if let Some(entry) = guard.sessions.get_mut(&session_key) {
        entry.uris.remove(uri);
        if entry.uris.is_empty() {
            guard.sessions.remove(&session_key);
        }
    }
}

pub async fn notify_resource_updated(uri: &str) {
    let peers = {
        let mut guard = SUBSCRIPTIONS.lock().expect("subscription registry lock");
        prune_closed_subscriptions_locked(&mut guard);

        guard
            .sessions
            .values()
            .filter(|entry| entry.uris.contains(uri))
            .map(|entry| entry.peer.clone())
            .collect::<Vec<_>>()
    };

    for peer in peers {
        let _ = peer
            .notify_resource_updated(ResourceUpdatedNotificationParam::new(uri))
            .await;
    }
}

pub fn asset_uri_from_path(
    assets_root: &std::path::Path,
    path: &std::path::Path,
) -> Option<String> {
    let relative = path.strip_prefix(assets_root).ok()?;
    let mut parts = relative.components();
    let kind = parts.next()?.as_os_str().to_string_lossy();
    let file = parts.next()?;
    if parts.next().is_some() {
        return None;
    }

    let id = std::path::Path::new(file.as_os_str())
        .file_stem()?
        .to_string_lossy();

    Some(format!("asset://{kind}/{id}"))
}

pub fn prune_closed_subscriptions() {
    let mut guard = SUBSCRIPTIONS.lock().expect("subscription registry lock");
    prune_closed_subscriptions_locked(&mut guard);
}

fn prune_closed_subscriptions_locked(registry: &mut SubscriptionRegistry) {
    registry
        .sessions
        .retain(|_, session| !session.peer.is_transport_closed() && !session.uris.is_empty());
}

fn session_key(context: &RequestContext<RoleServer>) -> String {
    if let Some(parts) = context.extensions.get::<HttpRequestParts>() {
        if let Some(value) = parts
            .headers
            .get(HEADER_SESSION_ID)
            .and_then(|header| header.to_str().ok())
        {
            return format!("http:{value}");
        }
        if let Some(addr) = parts.extensions.get::<std::net::SocketAddr>() {
            return format!("http-remote:{addr}");
        }
        return "http-stateless".to_string();
    }
    "stdio".to_string()
}
