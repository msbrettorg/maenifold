use crate::community;
use crate::config::Config;
use crate::db::{self, GraphDatabase};
use crate::error::{MaenifoldError, Result};
use crate::json::OutputFormat;
use crate::leader::LeaderElection;
use crate::markdown::{
    Frontmatter, extract_wikilinks, extract_wikilinks_with_sections, path_to_uri, read_markdown,
};
use crate::sessions;
use crate::tools::ToolContext;
use chrono::{DateTime, Utc};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};

static WATCHER: Lazy<Mutex<Option<WatcherState>>> = Lazy::new(|| Mutex::new(None));

struct WatcherState {
    watcher: notify::RecommendedWatcher,
    control_tx: Sender<WatcherSignal>,
    worker: Option<JoinHandle<()>>,
}

impl WatcherState {
    fn stop(mut self) {
        let _ = self.control_tx.send(WatcherSignal::Stop);
        drop(self.watcher);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

enum WatcherSignal {
    Event(Event),
    Stop,
}

struct IndexedFileState {
    last_indexed: Option<String>,
    file_md5: Option<String>,
    has_mentions: bool,
}

type RelationKey = (String, String);
type RelationSourceWeights = BTreeMap<String, f64>;
type RelationCounts = BTreeMap<RelationKey, (f64, RelationSourceWeights)>;

pub fn start_watcher(context: &ToolContext, payload: &Value) -> Result<String> {
    let debounce_ms = payload
        .get("debounceMs")
        .and_then(Value::as_u64)
        .or_else(read_debounce_ms_from_env)
        .unwrap_or(500);
    let config = context.config.clone();
    let holder_id = generate_sync_holder_id();

    if WATCHER.lock().is_some() {
        return Ok("Watcher already running".to_string());
    }

    let (event_tx, event_rx) = mpsc::channel();
    let worker = spawn_worker(config.clone(), holder_id, debounce_ms, event_rx);
    let callback_tx = event_tx.clone();

    let mut watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
        let Ok(event) = event else {
            if let Err(error) = event {
                eprintln!("{}", format_watcher_error_message(&error.to_string()));
            }
            return;
        };
        if !is_relevant_event_kind(&event.kind) {
            return;
        }
        let _ = callback_tx.send(WatcherSignal::Event(event));
    })
    .map_err(map_notify_error)?;

    if let Err(error) = watcher.watch(&config.memory_path, RecursiveMode::Recursive) {
        let _ = event_tx.send(WatcherSignal::Stop);
        let _ = worker.join();
        return Err(map_notify_error(error));
    }

    WATCHER.lock().replace(WatcherState {
        watcher,
        control_tx: event_tx,
        worker: Some(worker),
    });

    Ok(format!(
        "Started watching {} for incremental sync (debounce: {}ms)",
        context.config.memory_path.display(),
        debounce_ms,
    ))
}

pub fn stop_watcher(_context: &ToolContext, _payload: &Value) -> Result<String> {
    if let Some(state) = WATCHER.lock().take() {
        state.stop();
        return Ok("Stopped incremental sync watcher".to_string());
    }
    Ok("Watcher not running".to_string())
}

// T-0.1: RTM NFR-3 — preserve the registry-level StopWatcher contract expected by older boundary tests.
pub fn stop_watcher_tool(context: &ToolContext, payload: &Value) -> Result<String> {
    let stop_result = stop_watcher(context, payload)?;
    if stop_result == "Stopped incremental sync watcher" {
        return Ok("Stopped watcher".to_string());
    }
    Ok(stop_result)
}

// T-COV-001.3: RTM FR-17.5 — expose watcher state so integration tests can verify lifecycle guards.
#[doc(hidden)]
pub fn watcher_is_running() -> bool {
    WATCHER.lock().is_some()
}

// T-COV-001.3: RTM FR-17.5 — expose private incremental path processing so integration tests can port C# handler coverage.
#[doc(hidden)]
pub fn process_path_for_test(context: &ToolContext, path: &Path) -> Result<()> {
    if path_to_memory_uri(&context.config, path)?.is_none() {
        return Ok(());
    }
    crate::search::sync_single_file(context, path)?;
    Ok(())
}

// T-COV-001.3: RTM FR-17.5 — expose debounced batch semantics so integration tests can port watcher coalescing coverage deterministically.
#[doc(hidden)]
pub fn process_pending_paths_for_test(context: &ToolContext, paths: &[PathBuf]) -> Result<()> {
    let mut unique_paths = HashSet::new();
    for path in paths {
        if path_to_memory_uri(&context.config, path)?.is_some() {
            unique_paths.insert(path.clone());
        }
    }

    let paths = unique_paths.into_iter().collect::<Vec<_>>();
    process_incremental_batch(&context.config, &paths)
}

// T-COV-001.3: RTM FR-17.5 — keep watcher error text stable for parity-style assertions in integration tests.
#[doc(hidden)]
pub fn format_watcher_error_message(error: &str) -> String {
    format!("[FileWatcher] Error: {error}")
}

fn spawn_worker(
    config: Config,
    holder_id: String,
    debounce_ms: u64,
    event_rx: Receiver<WatcherSignal>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let debounce_window = Duration::from_millis(debounce_ms.max(1));
        let flush_grace_window = debounce_grace_window(debounce_window);
        let mut pending_paths = HashSet::new();

        loop {
            match event_rx.recv() {
                Ok(WatcherSignal::Event(event)) => {
                    collect_pending_paths(&config.memory_path, &event, &mut pending_paths);
                }
                Ok(WatcherSignal::Stop) | Err(_) => {
                    flush_pending_paths(&config, &holder_id, &mut pending_paths);
                    break;
                }
            }

            loop {
                match event_rx.recv_timeout(debounce_window) {
                    Ok(WatcherSignal::Event(event)) => {
                        collect_pending_paths(&config.memory_path, &event, &mut pending_paths);
                    }
                    Ok(WatcherSignal::Stop) => {
                        flush_pending_paths(&config, &holder_id, &mut pending_paths);
                        return;
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        // T-0.1: RTM NFR-3 — allow one short grace window so late duplicate filesystem
                        // events from the same write burst still collapse into a single debounced batch.
                        match event_rx.recv_timeout(flush_grace_window) {
                            Ok(WatcherSignal::Event(event)) => {
                                collect_pending_paths(&config.memory_path, &event, &mut pending_paths);
                            }
                            Ok(WatcherSignal::Stop) => {
                                flush_pending_paths(&config, &holder_id, &mut pending_paths);
                                return;
                            }
                            Err(RecvTimeoutError::Timeout) => {
                                flush_pending_paths(&config, &holder_id, &mut pending_paths);
                                break;
                            }
                            Err(RecvTimeoutError::Disconnected) => {
                                flush_pending_paths(&config, &holder_id, &mut pending_paths);
                                return;
                            }
                        }
                    }
                    Err(RecvTimeoutError::Disconnected) => {
                        flush_pending_paths(&config, &holder_id, &mut pending_paths);
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

fn collect_pending_paths(memory_path: &Path, event: &Event, pending_paths: &mut HashSet<PathBuf>) {
    for path in &event.paths {
        if !is_watchable_markdown_path(memory_path, path) {
            continue;
        }
        pending_paths.insert(path.clone());
    }
}

fn flush_pending_paths(config: &Config, holder_id: &str, pending_paths: &mut HashSet<PathBuf>) {
    if pending_paths.is_empty() {
        return;
    }

    let paths = pending_paths.drain().collect::<Vec<_>>();
    if let Err(error) = process_pending_paths(config, holder_id, &paths) {
        eprintln!("maenifold watcher incremental sync failed: {error}");
    }
}

fn process_pending_paths(config: &Config, holder_id: &str, paths: &[PathBuf]) -> Result<()> {
    let role = sync_role();
    // T-4.1: RTM FR-30.6
    if role == "follower" {
        eprintln!("[maenifold] watcher: follower role, skipping sync");
        return Ok(());
    }

    let leader_election = LeaderElection::new(&config.database_path);
    // T-5.4: RTM FR-30.4 / FR-30.9
    let (lease_owned, reason) = if role == "leader" {
        leader_election.force_acquire(holder_id, 60)?
    } else {
        // T-5.4: RTM FR-30.1
        leader_election.try_acquire(holder_id, 60)?
    };
    // T-4.1: RTM FR-30.6
    if !lease_owned {
        eprintln!(
            "[maenifold] watcher: another process is leader, skipping sync (reason: {})",
            reason
        );
        return Ok(());
    }

    GraphDatabase::initialize_database(config)?;
    let result = process_incremental_batch(config, paths);
    let _ = leader_election.release(holder_id);
    result
}

fn process_incremental_batch(config: &Config, paths: &[PathBuf]) -> Result<()> {
    let mut sorted_paths = paths.to_vec();
    sorted_paths.sort();
    let context = ToolContext::new(config.clone(), OutputFormat::Markdown);

    for path in sorted_paths {
        match crate::search::sync_single_file(&context, &path) {
            Ok(_) => {}
            Err(error) => {
                // T-5.8: RTM NFR-3, NFR-4 isolates watcher batch failures per file.
                eprintln!(
                    "maenifold watcher failed to process {}: {error}",
                    path.display()
                );
            }
        }
    }

    let _communities_detected = crate::search::recompute_communities_if_needed(&context)?;
    let _abandoned_sessions = sessions::cleanup_abandoned_sessions(config)?;
    Ok(())
}

fn process_incremental_path(
    connection: &Connection,
    config: &Config,
    path: &Path,
    sync_started_at: &str,
) -> Result<()> {
    let Some(file_uri) = path_to_memory_uri(config, path)? else {
        return Ok(());
    };

    if path.exists() {
        // T-5.8: RTM NFR-3, NFR-4 rejects symlinked watcher inputs and only processes regular files.
        let metadata = fs::symlink_metadata(path)?;
        let file_type = metadata.file_type();
        if file_type.is_symlink() || !file_type.is_file() {
            return Ok(());
        }

        let stored_file = load_indexed_file_state(connection, &file_uri)?;
        let synced_file = stored_file
            .as_ref()
            .filter(|indexed_file| indexed_file.has_mentions);
        let stored_hash = synced_file.and_then(|indexed_file| indexed_file.file_md5.as_deref());
        let last_indexed =
            synced_file.and_then(|indexed_file| indexed_file.last_indexed.as_deref());

        if should_skip_by_mtime(metadata.modified().ok(), last_indexed) {
            return Ok(());
        }

        let (frontmatter, content, checksum) = read_markdown(&file_uri, config)?;
        if stored_hash == Some(checksum.as_str()) {
            connection.execute(
                "UPDATE file_content SET last_indexed = ?2 WHERE file_path = ?1",
                params![file_uri, sync_started_at],
            )?;
            return Ok(());
        }

        // T-5.4: RTM FR-30.1
        upsert_processed_file(
            connection,
            &file_uri,
            &frontmatter,
            &content,
            &checksum,
            metadata.len() as i64,
            sync_started_at,
        )?;
    } else {
        // T-5.4: RTM FR-30.1
        tx_delete_file_index(connection, &file_uri)?;
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

fn is_relevant_event_kind(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Any | EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

fn is_watchable_markdown_path(memory_path: &Path, path: &Path) -> bool {
    if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
        return false;
    }
    if !path.starts_with(memory_path) {
        return false;
    }
    !is_hidden_relative_path(memory_path, path)
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

fn read_debounce_ms_from_env() -> Option<u64> {
    std::env::var("MAENIFOLD_DEBOUNCE_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
}

fn sync_role() -> String {
    match std::env::var("MAENIFOLD_SYNC_ROLE") {
        Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
            "leader" => "leader".to_string(),
            "follower" => "follower".to_string(),
            _ => "auto".to_string(),
        },
        Err(_) => "auto".to_string(),
    }
}

fn generate_sync_holder_id() -> String {
    format!("watcher-{}-{}", std::process::id(), uuid::Uuid::new_v4())
}

fn current_iso_timestamp() -> String {
    Utc::now().to_rfc3339()
}

fn map_notify_error(error: notify::Error) -> MaenifoldError {
    MaenifoldError::Message(format!("watcher error: {error}"))
}

fn load_indexed_file_state(
    connection: &Connection,
    file_uri: &str,
) -> Result<Option<IndexedFileState>> {
    Ok(connection
        .query_row(
            "SELECT
                 last_indexed,
                 file_md5,
                 EXISTS(SELECT 1 FROM concept_mentions WHERE source_file = file_content.file_path)
             FROM file_content
             WHERE file_path = ?1",
            params![file_uri],
            |row| {
                Ok(IndexedFileState {
                    last_indexed: row.get(0)?,
                    file_md5: row.get(1)?,
                    has_mentions: row.get::<_, i64>(2)? != 0,
                })
            },
        )
        .optional()?)
}

fn tx_delete_file_index(connection: &Connection, file_uri: &str) -> Result<()> {
    connection.execute(
        "DELETE FROM concept_mentions WHERE source_file = ?1",
        params![file_uri],
    )?;
    connection.execute(
        "DELETE FROM file_content WHERE file_path = ?1",
        params![file_uri],
    )?;
    Ok(())
}

fn should_skip_by_mtime(modified_at: Option<SystemTime>, last_indexed: Option<&str>) -> bool {
    let Some(modified_at) = modified_at else {
        return false;
    };
    let Some(last_indexed) = last_indexed else {
        return false;
    };
    let Some(last_indexed) = parse_indexed_timestamp(last_indexed) else {
        return false;
    };

    let modified_at: DateTime<Utc> = modified_at.into();
    modified_at <= last_indexed
}

fn parse_indexed_timestamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|parsed| parsed.with_timezone(&Utc))
}

fn upsert_processed_file(
    connection: &Connection,
    uri: &str,
    frontmatter: &Option<Frontmatter>,
    content: &str,
    checksum: &str,
    file_size: i64,
    indexed_at: &str,
) -> Result<()> {
    let title = frontmatter
        .as_ref()
        .and_then(|frontmatter| frontmatter.get("title"))
        .and_then(|value| value.as_str())
        .unwrap_or_else(|| {
            uri.trim_start_matches("memory://")
                .split('/')
                .next_back()
                .unwrap_or_default()
        })
        .to_string();
    let status = frontmatter
        .as_ref()
        .and_then(|frontmatter| frontmatter.get("status"))
        .and_then(|value| value.as_str())
        .unwrap_or("saved")
        .to_string();
    let created = frontmatter
        .as_ref()
        .and_then(|frontmatter| frontmatter.get("created"))
        .and_then(|value| value.as_str())
        .map(str::to_string);

    connection.execute(
        "INSERT INTO file_content (file_path, title, content, summary, last_indexed, status, file_md5, file_size, created, last_accessed)
         VALUES (?1, ?2, ?3, '', ?4, ?5, ?6, ?7, COALESCE(?8, ?4), COALESCE((SELECT last_accessed FROM file_content WHERE file_path = ?1), COALESCE(?8, ?4)))
         ON CONFLICT(file_path) DO UPDATE SET title=excluded.title, content=excluded.content, summary=excluded.summary, last_indexed=excluded.last_indexed, status=excluded.status, file_md5=excluded.file_md5, file_size=excluded.file_size, created=COALESCE(file_content.created, excluded.created), last_accessed=COALESCE(file_content.last_accessed, excluded.last_accessed)",
        params![uri, title, content, indexed_at, status, checksum, file_size, created],
    )?;

    connection.execute(
        "DELETE FROM concept_mentions WHERE source_file = ?1",
        params![uri],
    )?;

    let mut per_file = BTreeMap::<String, usize>::new();
    for concept in extract_wikilinks(content) {
        *per_file.entry(concept.to_ascii_lowercase()).or_default() += 1;
    }

    for (concept, mention_count) in per_file {
        connection.execute(
            "INSERT OR IGNORE INTO concepts (concept_name, first_seen, occurrence_count) VALUES (?1, ?2, 0)",
            params![concept, indexed_at],
        )?;
        connection.execute(
            "INSERT INTO concept_mentions (concept_name, source_file, mention_count) VALUES (?1, ?2, ?3)
             ON CONFLICT(concept_name, source_file) DO UPDATE SET mention_count = excluded.mention_count",
            params![concept, uri, mention_count as i64],
        )?;
    }

    Ok(())
}

fn rebuild_graph_from_mentions(connection: &Connection, indexed_at: &str) -> Result<i64> {
    let existing_first_seen = load_existing_first_seen(connection)?;
    let mut statement = connection.prepare(
        "SELECT source_file, concept_name, mention_count FROM concept_mentions ORDER BY source_file, concept_name",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })?;

    let mut total_mentions = 0i64;
    let mut concept_counts = BTreeMap::<String, i64>::new();
    for row in rows {
        let (_source_file, concept_name, mention_count) = row?;
        total_mentions += mention_count;
        *concept_counts.entry(concept_name.clone()).or_default() += mention_count;
    }

    let relation_counts = load_relation_counts(connection)?;
    connection.execute("DELETE FROM concept_graph", [])?;
    connection.execute("DELETE FROM concepts", [])?;

    for (concept_name, occurrence_count) in concept_counts {
        let first_seen = existing_first_seen
            .get(&concept_name)
            .cloned()
            .unwrap_or_else(|| indexed_at.to_string());
        connection.execute(
            "INSERT INTO concepts (concept_name, first_seen, occurrence_count) VALUES (?1, ?2, ?3)",
            params![concept_name, first_seen, occurrence_count],
        )?;
    }

    for ((concept_a, concept_b), (count, files)) in relation_counts {
        let source_files = serde_json::to_string(&files)?;
        connection.execute(
            "INSERT INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files) VALUES (?1, ?2, ?3, ?4)",
            params![concept_a, concept_b, count, source_files],
        )?;
    }

    Ok(total_mentions)
}

fn load_existing_first_seen(connection: &Connection) -> Result<BTreeMap<String, String>> {
    let mut statement = connection
        .prepare("SELECT concept_name, first_seen FROM concepts WHERE first_seen IS NOT NULL")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    Ok(rows.collect::<std::result::Result<BTreeMap<_, _>, _>>()?)
}

fn load_relation_counts(connection: &Connection) -> Result<RelationCounts> {
    let mut statement =
        connection.prepare("SELECT file_path, content FROM file_content ORDER BY file_path")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut relation_counts = RelationCounts::new();
    for row in rows {
        let (source_file, content) = row?;
        flush_relation_counts(&source_file, &content, &mut relation_counts);
    }

    Ok(relation_counts)
}

fn flush_relation_counts(source_file: &str, content: &str, relation_counts: &mut RelationCounts) {
    let mut concepts_by_section = BTreeMap::<String, BTreeSet<usize>>::new();
    for (concept, section_index) in extract_wikilinks_with_sections(content) {
        concepts_by_section
            .entry(concept)
            .or_default()
            .insert(section_index);
    }

    let concepts = concepts_by_section.into_iter().collect::<Vec<_>>();
    for index in 0..concepts.len() {
        for other_index in index + 1..concepts.len() {
            let (first, first_sections) = &concepts[index];
            let (second, second_sections) = &concepts[other_index];
            let key = if first <= second {
                (first.clone(), second.clone())
            } else {
                (second.clone(), first.clone())
            };
            let weight = if first_sections
                .iter()
                .any(|section_index| second_sections.contains(section_index))
            {
                1.0
            } else {
                0.5
            };
            let entry = relation_counts
                .entry(key)
                .or_insert_with(|| (0.0, BTreeMap::new()));
            entry.0 += weight;
            entry
                .1
                .entry(source_file.to_string())
                .and_modify(|existing| *existing = existing.max(weight))
                .or_insert(weight);
        }
    }
}
