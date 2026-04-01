use crate::config::Config;
use crate::db::{self, GraphDatabase};
use crate::error::Result;
use crate::markdown::path_to_uri;
use crate::sessions;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use notify::{Event, EventKind};
use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use super::leader::LeaderElection;
use super::sync_state::sync_role;

#[doc(hidden)]
pub fn process_path_for_test(context: &ToolContext, path: &Path) -> Result<()> {
    if path_to_memory_uri(&context.config, path)?.is_none() {
        return Ok(());
    }
    crate::sync::sync_single_file(context, path)?;
    Ok(())
}

#[doc(hidden)]
pub fn process_pending_paths_for_test(context: &ToolContext, paths: &[PathBuf]) -> Result<()> {
    let mut unique_paths = HashSet::new();
    for path in paths {
        if path_to_memory_uri(&context.config, path)?.is_some() {
            unique_paths.insert(path.clone());
        }
    }
    process_incremental_batch(
        &context.config,
        &unique_paths.into_iter().collect::<Vec<_>>(),
    )
}

#[doc(hidden)]
pub fn format_watcher_error_message(error: &str) -> String {
    format!("[FileWatcher] Error: {error}")
}

pub(super) fn is_relevant_event_kind(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Any | EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

pub(super) fn collect_pending_paths(
    memory_path: &Path,
    event: &Event,
    pending_paths: &mut HashSet<PathBuf>,
) {
    for path in &event.paths {
        if is_watchable_markdown_path(memory_path, path) {
            pending_paths.insert(path.clone());
        }
    }
}

pub(super) fn flush_pending_paths(
    config: &Config,
    holder_id: &str,
    pending_paths: &mut HashSet<PathBuf>,
) {
    if pending_paths.is_empty() {
        return;
    }

    let paths = pending_paths.drain().collect::<Vec<_>>();
    if let Err(error) = process_pending_paths(config, holder_id, &paths) {
        super::watcher_health::record_file_failure(error.to_string());
        eprintln!("maenifold watcher incremental sync failed: {error}");
    }
}

fn process_pending_paths(config: &Config, holder_id: &str, paths: &[PathBuf]) -> Result<()> {
    let role = sync_role();
    if role == "follower" {
        eprintln!("[maenifold] watcher: follower role, skipping sync");
        return Ok(());
    }

    let leader_election = LeaderElection::new(&config.database_path);
    let (lease_owned, reason, epoch) = if role == "leader" {
        leader_election.force_acquire(holder_id, config.sync_lease_ms())?
    } else {
        leader_election.try_acquire(holder_id, config.sync_lease_ms())?
    };
    if !lease_owned {
        eprintln!(
            "[maenifold] watcher: another process is leader, skipping sync (reason: {})",
            reason
        );
        return Ok(());
    }

    GraphDatabase::initialize_database(config)?;
    let result = process_incremental_batch(config, paths);
    let _ = leader_election.release(holder_id, epoch);
    result
}

fn process_incremental_batch(config: &Config, paths: &[PathBuf]) -> Result<()> {
    let mut sorted_paths = paths.to_vec();
    sorted_paths.sort();
    let context = ToolContext::new(config.clone(), OutputFormat::Markdown);
    let mut had_file_error = false;

    for path in &sorted_paths {
        if let Err(error) = crate::sync::sync_single_file(&context, path) {
            had_file_error = true;
            super::watcher_health::record_file_failure(format!("{}: {error}", path.display()));
            eprintln!(
                "maenifold watcher failed to process {}: {error}",
                path.display()
            );
        }
    }

    let orphan_paths = sorted_paths
        .iter()
        .filter(|path| !path.exists())
        .filter_map(|path| path_to_memory_uri(config, path).ok().flatten())
        .collect::<BTreeSet<_>>();
    if !orphan_paths.is_empty() {
        match db::open_connection(&config.database_path) {
            Ok(connection) => {
                let table_exists: bool = connection
                    .query_row(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'vec_memory_files'",
                        [],
                        |row| row.get::<_, i64>(0),
                    )
                    .map(|count| count > 0)
                    .unwrap_or(false);
                if table_exists
                    && let Err(error) =
                        crate::search::cleanup_orphan_vec_memory_files(&connection, &orphan_paths)
                {
                    eprintln!("[SYNC] orphan vector cleanup failed: {error}");
                }
            }
            Err(error) => eprintln!("[SYNC] failed to open database for orphan cleanup: {error}"),
        }
    }

    let _communities_detected = crate::sync::recompute_communities_if_needed(&context)?;
    let _abandoned_sessions = sessions::cleanup_abandoned_sessions(config)?;
    super::watcher_health::schedule_maintenance_if_needed(config, sorted_paths.len() as u32);
    if !had_file_error {
        super::watcher_health::clear_consecutive_failures();
    }
    Ok(())
}

fn path_to_memory_uri(config: &Config, path: &Path) -> Result<Option<String>> {
    if !path.starts_with(&config.memory_path) {
        return Ok(None);
    }
    if !path.exists() && path.extension().and_then(|ext| ext.to_str()) != Some("md") {
        return Ok(None);
    }
    if !path.exists() && is_hidden_relative_path(&config.memory_path, path) {
        return Ok(None);
    }
    if path.exists() && !is_watchable_markdown_path(&config.memory_path, path) {
        return Ok(None);
    }
    path_to_uri(path, &config.memory_path).map(Some)
}

pub(crate) fn is_watchable_markdown_path(memory_path: &Path, path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()) == Some("md")
        && path.starts_with(memory_path)
        && !is_hidden_relative_path(memory_path, path)
}

fn is_hidden_relative_path(memory_path: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(memory_path) else {
        return true;
    };
    relative.components().any(|component| {
        let text = component.as_os_str().to_string_lossy();
        text.starts_with('.') && text.len() > 1
    })
}
