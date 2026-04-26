pub mod cache;
pub mod endophasia;
pub mod image_client;
pub mod service;
pub mod zeitgeist;

use once_cell::sync::Lazy;
use parking_lot::RwLock;

pub struct WallpaperServiceState {
    pub last_generated: Option<chrono::DateTime<chrono::Utc>>,
    pub error: Option<String>,
}

static SERVICE_STATE: Lazy<RwLock<WallpaperServiceState>> = Lazy::new(|| {
    RwLock::new(WallpaperServiceState {
        last_generated: None,
        error: None,
    })
});

pub fn service_state() -> &'static RwLock<WallpaperServiceState> {
    &SERVICE_STATE
}
