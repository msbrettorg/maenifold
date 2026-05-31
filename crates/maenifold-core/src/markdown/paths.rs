use super::wikilinks::slugify;
use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use crate::util::paths::join_validated_relative_path;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

const FILE_READ_TIMEOUT: Duration = Duration::from_secs(30);

pub fn validate_uri(uri: &str, param_name: &str) -> Result<()> {
    if uri.is_empty() {
        return Err(MaenifoldError::InvalidArgument(format!(
            "{param_name}: URI cannot be null or empty."
        )));
    }
    if !uri.starts_with("memory://") {
        return Err(MaenifoldError::InvalidArgument(format!(
            "{param_name}: URI must start with 'memory://'. Got: '{uri}'"
        )));
    }
    let relative_part = &uri["memory://".len()..];
    if relative_part.split('/').any(|segment| segment == "..") {
        return Err(MaenifoldError::InvalidArgument(format!(
            "{param_name}: URI contains path traversal (..) which is not allowed: '{uri}'"
        )));
    }
    Ok(())
}

pub fn path_to_uri(path: &Path, base_path: &Path) -> Result<String> {
    let relative = path.strip_prefix(base_path).map_err(|_| {
        MaenifoldError::InvalidArgument(format!("Path is outside memory root: {}", path.display()))
    })?;
    let mut relative = relative.to_string_lossy().replace('\\', "/");
    if relative.ends_with(".md") {
        relative.truncate(relative.len() - 3);
    }
    Ok(format!("memory://{relative}"))
}

/// Returns true if the path already ends with `.md` (case-insensitive).
/// T-15.19: RTM NFR-1 - C# checks `EndsWithOrdinalIgnoreCase(".md")`, not "has no extension".
fn has_md_extension(path: &Path) -> bool {
    path.as_os_str()
        .to_string_lossy()
        .to_ascii_lowercase()
        .ends_with(".md")
}

pub fn uri_to_path(uri: &str, base_path: &Path) -> Result<PathBuf> {
    validate_uri(uri, "uri")?;
    // T-14.5: RTM FR-12 - canonicalize joined memory paths to block symlink traversal outside root.
    let mut path = join_validated_relative_path(base_path, uri.trim_start_matches("memory://"))?;
    // T-15.19: RTM NFR-1 - append .md unless already ends with .md (case-insensitive), matching C# UriToPath.
    if !has_md_extension(&path) {
        let mut name = path.file_name().unwrap_or_default().to_os_string();
        name.push(".md");
        path.set_file_name(name);
    }

    Ok(path)
}

pub fn file_exists(uri: &str, config: &Config) -> Result<bool> {
    Ok(uri_to_path(uri, &config.memory_path)?.exists())
}

pub fn delete_file(uri: &str, config: &Config) -> Result<()> {
    let path = uri_to_path(uri, &config.memory_path)?;
    if !path.exists() {
        return Err(MaenifoldError::Message(format!(
            "Memory file not found: {uri}"
        )));
    }
    fs::remove_file(path)?;
    Ok(())
}

pub fn read_to_string_guarded(path: &Path) -> Result<String> {
    let metadata = fs::metadata(path)?;
    if is_dataless(&metadata) {
        return Err(MaenifoldError::FileDataless {
            path: path.to_path_buf(),
            size: metadata.len(),
        });
    }
    read_to_string_with_timeout(path, FILE_READ_TIMEOUT)
}

#[cfg(all(test, unix))]
pub(crate) fn read_to_string_guarded_for_test(path: &Path, timeout: Duration) -> Result<String> {
    let metadata = fs::metadata(path)?;
    if is_dataless(&metadata) {
        return Err(MaenifoldError::FileDataless {
            path: path.to_path_buf(),
            size: metadata.len(),
        });
    }
    read_to_string_with_timeout(path, timeout)
}

pub fn is_dataless(metadata: &fs::Metadata) -> bool {
    is_dataless_size_blocks(metadata.len(), metadata_blocks(metadata))
}

pub fn is_dataless_size_blocks(size: u64, blocks: u64) -> bool {
    blocks == 0 && size > 0
}

fn metadata_blocks(metadata: &fs::Metadata) -> u64 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        metadata.blocks()
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        1
    }
}

fn read_to_string_with_timeout(path: &Path, timeout: Duration) -> Result<String> {
    let (sender, receiver) = mpsc::sync_channel(1);
    let path = path.to_path_buf();
    let handle = std::thread::Builder::new()
        .name("maenifold-file-read-timeout".to_string())
        .spawn({
            let path = path.clone();
            move || {
                let _ = sender.send(read_to_string_with_tokio_timeout(path, timeout));
            }
        })
        .map_err(|error| {
            MaenifoldError::Message(format!(
                "file read worker failed: {}: {error}",
                path.display()
            ))
        })?;
    let result = receiver.recv().map_err(|error| {
        MaenifoldError::Message(format!(
            "file read worker failed: {}: {error}",
            path.display()
        ))
    })?;
    let _ = handle.join();
    result
}

fn read_to_string_with_tokio_timeout(path: PathBuf, timeout: Duration) -> Result<String> {
    let path_text = path.display().to_string();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .map_err(|error| {
            MaenifoldError::Message(format!("file read runtime failed: {path_text}: {error}"))
        })?;
    let result = runtime.block_on(async {
        let read_path = path.clone();
        match tokio::time::timeout(
            timeout,
            tokio::task::spawn_blocking(move || fs::read_to_string(read_path)),
        )
        .await
        {
            Ok(join_result) => join_result
                .map_err(|error| {
                    MaenifoldError::Message(format!("file read task failed: {path_text}: {error}"))
                })?
                .map_err(MaenifoldError::from),
            Err(_) => Err(file_read_timeout_error(&path)),
        }
    });
    runtime.shutdown_timeout(Duration::from_millis(10));
    result
}

fn file_read_timeout_error(path: &Path) -> MaenifoldError {
    MaenifoldError::Message(format!(
        "file read timed out: {} (likely cloud-offloaded — run `brctl download` or move maenifold root to local-only storage)",
        path.display()
    ))
}

pub fn get_memory_files(config: &Config, folder_uri: Option<&str>) -> Result<Vec<String>> {
    let base = if let Some(folder_uri) = folder_uri {
        validate_uri(folder_uri, "folderUri")?;
        join_validated_relative_path(
            &config.memory_path,
            folder_uri.trim_start_matches("memory://"),
        )?
    } else {
        config.memory_path.clone()
    };

    let mut files = Vec::new();
    if !base.exists() {
        return Ok(files);
    }

    for entry in walkdir::WalkDir::new(&base)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        if is_hidden_path(path, &config.memory_path) {
            continue;
        }
        files.push(path_to_uri(path, &config.memory_path)?);
    }

    files.sort();
    Ok(files)
}

fn is_hidden_path(path: &Path, base_path: &Path) -> bool {
    let relative = path.strip_prefix(base_path).unwrap_or(path);
    relative.components().any(|component| {
        let text = component.as_os_str().to_string_lossy();
        text.starts_with('.') && text.len() > 1
    })
}

pub fn resolve_folder_uri(folder_uri: &str, config: &Config) -> Result<PathBuf> {
    validate_uri(folder_uri, "folderUri")?;
    // T-14.5: RTM FR-12 - folder resolution must reject symlink escapes, including nonexistent leaves.
    join_validated_relative_path(
        &config.memory_path,
        folder_uri.trim_start_matches("memory://"),
    )
}

// T-12.0: RTM NFR-1 keeps note lookup behavior stable after the markdown split.
pub fn find_file_as_uri(identifier: &str, config: &Config) -> Option<String> {
    if identifier.trim().is_empty() {
        return None;
    }

    let normalized = identifier.replace('\\', "/").trim_matches('/').to_string();
    if normalized.is_empty() {
        return None;
    }

    let relative_path = normalized.replace('/', std::path::MAIN_SEPARATOR_STR);
    let direct_path = config.memory_path.join(&relative_path);
    if direct_path.is_file() {
        let uri_part = normalized.strip_suffix(".md").unwrap_or(&normalized);
        return Some(format!("memory://{uri_part}"));
    }

    if Path::new(&relative_path).extension().is_none() {
        let md_path = config.memory_path.join(format!("{relative_path}.md"));
        if md_path.is_file() {
            return Some(format!("memory://{normalized}"));
        }
    }

    let slug = slugify(identifier);
    get_memory_files(config, None)
        .ok()?
        .into_iter()
        .find(|uri| {
            uri.trim_start_matches("memory://")
                .split('/')
                .next_back()
                .map(|name| name.eq_ignore_ascii_case(&slug))
                .unwrap_or(false)
        })
}

pub fn markdown_path_from_relative(relative: &str, config: &Config) -> Result<PathBuf> {
    // T-14.5: RTM FR-12 - relative markdown paths must remain under memory root after symlink resolution.
    let mut path = join_validated_relative_path(&config.memory_path, relative)?;
    // T-15.19: RTM NFR-1 - append .md unless already ends with .md (case-insensitive), matching C# UriToPath.
    if !has_md_extension(&path) {
        let mut name = path.file_name().unwrap_or_default().to_os_string();
        name.push(".md");
        path.set_file_name(name);
    }
    Ok(path)
}
