use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;

pub fn cache_dir(memory_path: &Path) -> PathBuf {
    memory_path.join(".cache").join("wallpaper")
}

pub fn save_image(cache_dir: &Path, mode: &str, label: &str, data: &[u8]) -> Result<PathBuf> {
    fs::create_dir_all(cache_dir).with_context(|| {
        format!(
            "failed to create wallpaper cache dir {}",
            cache_dir.display()
        )
    })?;
    let timestamp = Utc::now().format("%Y%m%d%H%M%S%3f");
    let filename = format!(
        "{}_{}_{}.png",
        sanitize_filename_part(mode),
        sanitize_filename_part(label),
        timestamp
    );
    let path = cache_dir.join(filename);
    fs::write(&path, data)
        .with_context(|| format!("failed to write wallpaper image {}", path.display()))?;
    Ok(path)
}

pub fn latest_image(cache_dir: &Path, mode: &str) -> Option<PathBuf> {
    list_images(cache_dir, mode).pop()
}

pub fn latest_image_for(cache_dir: &Path, mode: &str, label: &str) -> Option<PathBuf> {
    let prefix = format!(
        "{}_{}_",
        sanitize_filename_part(mode),
        sanitize_filename_part(label)
    );
    list_images(cache_dir, mode)
        .into_iter()
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(&prefix))
        })
        .last()
}

pub fn list_images(cache_dir: &Path, mode: &str) -> Vec<PathBuf> {
    let prefix = format!("{}_", sanitize_filename_part(mode));
    let mut images = match fs::read_dir(cache_dir) {
        Ok(entries) => entries
            .filter_map(std::result::Result::ok)
            .map(|entry| entry.path())
            .filter(|path| is_mode_image(path, &prefix))
            .collect::<Vec<_>>(),
        Err(_) => Vec::new(),
    };
    images.sort_by_key(|path| image_timestamp(path).unwrap_or_default());
    images
}

pub fn clear_cache(cache_dir: &Path, mode: Option<&str>) -> Result<()> {
    if !cache_dir.exists() {
        return Ok(());
    }

    let mode_prefix = mode.map(|mode| format!("{}_", sanitize_filename_part(mode)));
    for entry in fs::read_dir(cache_dir)
        .with_context(|| format!("failed to read wallpaper cache dir {}", cache_dir.display()))?
    {
        let path = entry
            .with_context(|| format!("failed to inspect entry in {}", cache_dir.display()))?
            .path();
        let should_remove = mode_prefix
            .as_ref()
            .map_or_else(|| is_png(&path), |prefix| is_mode_image(&path, prefix));
        if should_remove {
            fs::remove_file(&path).with_context(|| {
                format!("failed to remove wallpaper cache file {}", path.display())
            })?;
        }
    }

    Ok(())
}

fn is_mode_image(path: &Path, prefix: &str) -> bool {
    is_png(path)
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(prefix))
}

fn is_png(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
}

fn image_timestamp(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .and_then(|stem| stem.rsplit_once('_'))
        .map(|(_, timestamp)| timestamp.to_string())
}

fn sanitize_filename_part(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "unknown".to_string()
    } else {
        sanitized
    }
}
