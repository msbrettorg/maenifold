use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use parking_lot::RwLock;
use tokio_util::sync::CancellationToken;

use crate::config::Config;
use crate::wallpaper::{endophasia, service_state, zeitgeist};

pub async fn wallpaper_service(config: Arc<RwLock<Config>>, shutdown: CancellationToken) {
    loop {
        let cfg = config.read().clone();
        let wallpaper = &cfg.settings.easter_eggs.wallpaper;

        if !wallpaper.enabled {
            if sleep_or_shutdown(&shutdown, Duration::from_secs(60)).await {
                break;
            }
            continue;
        }

        let sleep_duration = match wallpaper.mode.as_str() {
            "endophasia" => match endophasia::generate_endophasia_wallpaper(&cfg).await {
                Ok(_path) => {
                    let mut state = service_state().write();
                    state.last_generated = Some(Utc::now());
                    state.error = None;
                    Duration::from_secs(u64::from(wallpaper.rotate_minutes) * 60)
                }
                Err(error) => {
                    let message = error.to_string();
                    tracing::warn!(error = %message, "endophasia wallpaper generation failed");
                    service_state().write().error = Some(message);
                    Duration::from_secs(60)
                }
            },
            "zeitgeist" => match zeitgeist::generate_zeitgeist_wallpapers(&cfg).await {
                Ok(_paths) => {
                    let mut state = service_state().write();
                    state.last_generated = Some(Utc::now());
                    state.error = None;
                    Duration::from_secs(u64::from(wallpaper.rotate_minutes) * 60)
                }
                Err(error) => {
                    let message = error.to_string();
                    tracing::warn!(error = %message, "zeitgeist wallpaper generation failed");
                    service_state().write().error = Some(message);
                    Duration::from_secs(60)
                }
            },
            mode => {
                tracing::warn!(mode = %mode, "unsupported wallpaper mode");
                Duration::from_secs(u64::from(wallpaper.rotate_minutes) * 60)
            }
        };

        if sleep_or_shutdown(&shutdown, sleep_duration).await {
            break;
        }
    }
}

async fn sleep_or_shutdown(shutdown: &CancellationToken, duration: Duration) -> bool {
    tokio::select! {
        _ = shutdown.cancelled() => true,
        _ = tokio::time::sleep(duration) => false,
    }
}
