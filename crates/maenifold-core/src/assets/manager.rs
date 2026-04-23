// T-01.16: Facade for the assets::manager module.
//
// AssetManager state and constructor live here; behavior is implemented in
// sibling submodules. External callers continue to import `AssetManager` from
// `crate::assets::manager` (re-exported by `assets/mod.rs`).

mod init;
mod listing;
mod lookup;

#[cfg(test)]
mod tests;

use crate::config::Config;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AssetManager {
    pub(super) root: PathBuf,
}

impl AssetManager {
    pub fn new(config: &Config) -> Self {
        Self {
            root: config.assets_path.clone(),
        }
    }
}
