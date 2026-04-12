use crate::acp_config::{AgentsConfig, MaenifoldSettings};
use crate::api::config_manager::ConfigManager;
use crate::config::Config;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::stream::{self, Stream};
use notify::{Config as NotifyConfig, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use serde_json::json;
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;

#[derive(Clone)]
pub(crate) struct ConfigWatchState {
    sender: broadcast::Sender<ConfigWatchEvent>,
    startup_error: Option<Arc<str>>,
    _watcher: Arc<parking_lot::Mutex<Option<RecommendedWatcher>>>,
}

impl ConfigWatchState {
    pub(crate) fn new(
        config_manager: ConfigManager,
        config: Arc<parking_lot::RwLock<Config>>,
    ) -> Self {
        let (sender, _) = broadcast::channel(32);
        let (watcher, startup_error) =
            create_watcher(config_manager, config, sender.clone()).map_or_else(
                |error| {
                    tracing::warn!("config watch initialization failed: {error}");
                    (None, Some(Arc::<str>::from(error)))
                },
                |watcher| (Some(watcher), None),
            );

        Self {
            sender,
            startup_error,
            _watcher: Arc::new(parking_lot::Mutex::new(watcher)),
        }
    }

    pub(crate) fn startup_error(&self) -> Option<&str> {
        self.startup_error.as_deref()
    }

    fn subscribe(&self) -> broadcast::Receiver<ConfigWatchEvent> {
        self.sender.subscribe()
    }
}

#[derive(Debug, Clone, Serialize)]
struct ConfigWatchErrorEvent {
    path: String,
    message: String,
}

#[derive(Debug, Clone)]
enum ConfigWatchEvent {
    Changed(AgentsConfig),
    ReloadError(ConfigWatchErrorEvent),
    WatchError(ConfigWatchErrorEvent),
}

fn create_watcher(
    config_manager: ConfigManager,
    config: Arc<parking_lot::RwLock<Config>>,
    sender: broadcast::Sender<ConfigWatchEvent>,
) -> Result<RecommendedWatcher, String> {
    let config_path = config_manager.get_config_path();
    let config_path_for_watcher = config_path.clone();
    let config_path_for_errors = config_path.clone();

    let mut watcher = RecommendedWatcher::new(
        move |result: Result<notify::Event, notify::Error>| match result {
            Ok(event) => match event.kind {
                EventKind::Modify(_) | EventKind::Create(_) => {
                    if event.paths.iter().any(|path| path == &config_path_for_watcher) {
                        match config_manager.reload() {
                            Ok(new_config) => {
                                update_runtime_settings(&config, new_config.settings.as_ref());
                                let _ = sender.send(ConfigWatchEvent::Changed(new_config));
                            }
                            Err(error) => {
                                tracing::warn!(
                                    "failed to reload watched config '{}': {error}",
                                    config_path_for_watcher.display()
                                );
                                let _ = sender.send(ConfigWatchEvent::ReloadError(
                                    ConfigWatchErrorEvent {
                                        path: config_path_for_watcher.display().to_string(),
                                        message: error.to_string(),
                                    },
                                ));
                            }
                        }
                    }
                }
                _ => {}
            },
            Err(error) => {
                tracing::warn!(
                    "config watch error for '{}': {error}",
                    config_path_for_errors.display()
                );
                let _ = sender.send(ConfigWatchEvent::WatchError(ConfigWatchErrorEvent {
                    path: config_path_for_errors.display().to_string(),
                    message: error.to_string(),
                }));
            }
        },
        NotifyConfig::default(),
    )
    .map_err(|error| error.to_string())?;

    if let Some(parent) = config_path.parent() {
        watcher
            .watch(parent, RecursiveMode::NonRecursive)
            .map_err(|error| error.to_string())?;
    }

    Ok(watcher)
}

fn update_runtime_settings(
    config: &parking_lot::RwLock<Config>,
    settings: Option<&MaenifoldSettings>,
) {
    config.write().settings = settings.cloned().unwrap_or_default();
}

fn safe_json_event<T>(event_name: Option<&str>, payload: &T) -> Event
where
    T: Serialize,
{
    let mut event = Event::default();
    if let Some(event_name) = event_name {
        event = event.event(event_name);
    }

    match event.json_data(payload) {
        Ok(event) => event,
        Err(error) => {
            tracing::warn!("config SSE serialization failed: {error}");
            Event::default()
                .event("serialization-error")
                .json_data(json!({
                    "message": "config watch stream serialization failed"
                }))
                .unwrap_or_else(|fallback_error| {
                    tracing::warn!("config SSE fallback serialization failed: {fallback_error}");
                    Event::default().event("serialization-error")
                })
        }
    }
}

#[derive(Serialize)]
struct ConfigPathChangedEvent<'a> {
    path: &'a str,
    config: &'a AgentsConfig,
}

pub(crate) async fn config_watch_stream(
    config_watch: ConfigWatchState,
    config_path: PathBuf,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    config_watch_sse(config_watch.subscribe(), config_path.to_string_lossy().to_string())
}

fn config_watch_sse(
    receiver: broadcast::Receiver<ConfigWatchEvent>,
    path: String,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = stream::unfold(receiver, move |mut receiver| {
        let path = path.clone();
        async move {
            let event = match receiver.recv().await {
                Ok(ConfigWatchEvent::Changed(config)) => safe_json_event(
                    Some("config-changed"),
                    &ConfigPathChangedEvent {
                        path: &path,
                        config: &config,
                    },
                ),
                Ok(ConfigWatchEvent::ReloadError(error)) => {
                    safe_json_event(Some("config-error"), &error)
                }
                Ok(ConfigWatchEvent::WatchError(error)) => {
                    safe_json_event(Some("watch-error"), &error)
                }
                Err(RecvError::Lagged(skipped)) => {
                    safe_json_event(Some("lagged"), &json!({ "skipped": skipped }))
                }
                Err(RecvError::Closed) => return None,
            };

            Some((Ok(event), receiver))
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(30)).text("ping"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::response::IntoResponse;

    #[tokio::test]
    async fn config_watch_stream_emits_config_error_event() {
        let (sender, _) = broadcast::channel(4);
        let response =
            config_watch_sse(sender.subscribe(), "/tmp/maenifold.json".to_string()).into_response();

        sender
            .send(ConfigWatchEvent::ReloadError(ConfigWatchErrorEvent {
                path: "/tmp/maenifold.json".to_string(),
                message: "expected value at line 1 column 1".to_string(),
            }))
            .expect("send reload error");
        drop(sender);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("collect sse body");
        let body = String::from_utf8(body.to_vec()).expect("utf8 sse body");

        assert!(body.contains("event: config-error"), "{body}");
        assert!(body.contains(r#""path":"/tmp/maenifold.json""#), "{body}");
        assert!(
            body.contains(r#""message":"expected value at line 1 column 1""#),
            "{body}"
        );
    }
}
