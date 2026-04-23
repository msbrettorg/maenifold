// T-01.16: AssetManager initialization extracted from manager.rs.

use super::AssetManager;
use super::super::support::resolve_packaged_assets_root;
use crate::config::Config;
use crate::error::Result;
use std::fs;
use std::path::Path;

impl AssetManager {
    // T-ASSET-PATH: RTM FR-3, FR-20 — use runtime asset resolution instead
    // of compile-time CARGO_MANIFEST_DIR so the binary works on any machine.
    pub fn initialize_assets(config: &Config) -> Result<()> {
        config.ensure_directories()?;
        let packaged_root = match resolve_packaged_assets_root() {
            Some(p) => p,
            None => return Ok(()), // No packaged assets found — graceful
        };

        for entry in walkdir::WalkDir::new(&packaged_root)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            let src = entry.path();
            let relative = match src.strip_prefix(&packaged_root) {
                Ok(relative) if !relative.as_os_str().is_empty() => relative,
                _ => continue,
            };
            if should_skip_packaged_symlink_entry(src)? {
                continue;
            }
            let dst = config.assets_path.join(relative);
            if entry.file_type().is_dir() {
                fs::create_dir_all(&dst)?;
                continue;
            }
            if dst.exists() {
                continue;
            }
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }
            if let Err(error) = fs::copy(src, &dst) {
                if should_skip_initialize_asset_copy(src, &error) {
                    continue;
                }
                return Err(error.into());
            }
        }
        Ok(())
    }
}

fn should_skip_packaged_symlink_entry(src: &Path) -> Result<bool> {
    if !src.is_symlink() {
        return Ok(false);
    }

    match fs::metadata(src) {
        Ok(metadata) => Ok(metadata.is_dir()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(error) => Err(error.into()),
    }
}

fn should_skip_initialize_asset_copy(src: &Path, error: &std::io::Error) -> bool {
    if !src.is_symlink() {
        return false;
    }

    if error.kind() == std::io::ErrorKind::NotFound {
        return true;
    }

    if error.kind() != std::io::ErrorKind::InvalidInput {
        return false;
    }

    fs::metadata(src)
        .map(|metadata| metadata.is_dir())
        .unwrap_or(false)
}
