use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::Mutex;
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread::JoinHandle;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};

const GATE_WATCH_EVENT: &str = "gate://changed";
const GATE_WATCH_DEBOUNCE_MS: u64 = 150;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GateTaskRow {
    task_id: String,
    canonical_task_id: Option<String>,
    title: String,
    pipeline: Option<String>,
    state: String,
    stages: Vec<String>,
    attempts: u32,
    location: String,
    path: String,
    issue: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GateSnapshot {
    db_path: String,
    exists: bool,
    tasks: Vec<GateTaskRow>,
}

#[derive(Clone, Debug, Serialize)]
pub struct GateWatcherEvent {
    root: String,
}

#[derive(Default)]
pub struct GateWatcherManager(Mutex<Option<RunningGateWatcher>>);

struct RunningGateWatcher {
    watcher: RecommendedWatcher,
    stop_tx: Sender<()>,
    join_handle: JoinHandle<()>,
}

impl GateWatcherManager {
    fn replace(&self, next: RunningGateWatcher) {
        self.stop();
        *self.0.lock() = Some(next);
    }

    pub fn stop(&self) {
        if let Some(running) = self.0.lock().take() {
            let _ = running.stop_tx.send(());
            drop(running.watcher);
            let _ = running.join_handle.join();
        }
    }
}

fn gate_dir(root: &Path) -> PathBuf { root.join(".gate") }
fn gate_db_path(root: &Path) -> PathBuf { gate_dir(root).join("gate.db") }

fn is_gate_workspace(root: &Path) -> bool { root.join("todo").is_dir() || root.join("done").is_dir() }

fn gate_binary_candidates() -> Vec<PathBuf> {
    let mut paths = env::var_os("PATH")
        .into_iter()
        .flat_map(|value| env::split_paths(&value).collect::<Vec<_>>())
        .map(|path| path.join("gate"))
        .collect::<Vec<_>>();
    paths.extend([
        PathBuf::from("/usr/local/bin/gate"),
        PathBuf::from("/opt/homebrew/bin/gate"),
        PathBuf::from("/usr/bin/gate"),
    ]);
    paths
}

fn find_gate_binary() -> Result<PathBuf, String> {
    gate_binary_candidates()
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| "Could not find `gate` binary in PATH or common macOS locations".to_string())
}

fn bootstrap_gate_db(root: &Path) -> Result<(), String> {
    if gate_db_path(root).exists() || !is_gate_workspace(root) { return Ok(()); }

    let binary = find_gate_binary()?;
    let output = Command::new(binary)
        .arg("sync")
        .arg("--root")
        .arg(root)
        .output()
        .map_err(|e| format!("Failed to run `gate sync --root {}`: {e}", root.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        return Err(format!("Failed to bootstrap Gate read model for {}: {}", root.display(), detail));
    }

    if gate_db_path(root).exists() { Ok(()) } else { Err(format!("`gate sync` completed but {} was not created", gate_db_path(root).display())) }
}

fn open_snapshot_connection(root: &Path) -> Result<Option<(Connection, String)>, String> {
    bootstrap_gate_db(root)?;
    let db_path = gate_db_path(root);
    let db_path_string = db_path.to_string_lossy().to_string();
    if !db_path.exists() { return Ok(None); }
    let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)
        .map_err(|e| format!("Failed to open gate cache DB: {e}"))?;
    Ok(Some((conn, db_path_string)))
}

fn load_tasks(conn: &Connection, root: &Path) -> Result<Vec<GateTaskRow>, String> {
    let schema_version: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0)).unwrap_or(0);
    if schema_version < 2 { return Err(format!("Unsupported gate cache schema version {schema_version} at {}", gate_db_path(root).display())); }

    let mut stmt = conn.prepare(
        "SELECT task_id, canonical_task_id, title, pipeline, status, stages_json, attempts, location, file, issue
         FROM tasks
         ORDER BY CASE location WHEN 'todo' THEN 0 WHEN 'done' THEN 1 ELSE 2 END, task_id ASC",
    ).map_err(|e| format!("Failed to prepare gate cache query: {e}"))?;

    let rows = stmt.query_map([], |row| {
        let relative_path: String = row.get(8)?;
        let stages_json: String = row.get(5)?;
        let stages = serde_json::from_str::<Vec<String>>(&stages_json).map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(err))
        })?;
        Ok(GateTaskRow {
            task_id: row.get(0)?,
            canonical_task_id: row.get(1)?,
            title: row.get(2)?,
            pipeline: row.get(3)?,
            state: row.get(4)?,
            stages,
            attempts: row.get(6)?,
            location: row.get(7)?,
            path: root.join(relative_path).to_string_lossy().to_string(),
            issue: row.get(9)?,
        })
    }).map_err(|e| format!("Failed to read gate cache rows: {e}"))?;

    rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode gate cache rows: {e}"))
}

fn is_gate_watch_event(event: &Event) -> bool {
    event.paths.iter().any(|path| {
        matches!(
            path.file_name().and_then(|value| value.to_str()),
            Some(".gate" | "gate.db" | "gate.db-wal" | "gate.db-shm")
        )
    })
}

fn spawn_watch_thread(app: AppHandle, root: String, rx: mpsc::Receiver<Event>, stop_rx: mpsc::Receiver<()>) -> JoinHandle<()> {
    std::thread::spawn(move || loop {
        if stop_rx.try_recv().is_ok() { break; }
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(event) if is_gate_watch_event(&event) => {
                while stop_rx.try_recv().is_err() {
                    match rx.recv_timeout(Duration::from_millis(GATE_WATCH_DEBOUNCE_MS)) {
                        Ok(next) if is_gate_watch_event(&next) => continue,
                        Ok(_) | Err(RecvTimeoutError::Timeout) => break,
                        Err(RecvTimeoutError::Disconnected) => return,
                    }
                }
                if stop_rx.try_recv().is_ok() { break; }
                let _ = app.emit(GATE_WATCH_EVENT, GateWatcherEvent { root: root.clone() });
            }
            Ok(_) | Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    })
}

#[tauri::command]
pub fn load_gate_snapshot(root: String) -> Result<GateSnapshot, String> {
    let root_path = PathBuf::from(&root);
    if !root_path.is_absolute() { return Err("Gate root must be an absolute path".to_string()); }

    let db_path_string = gate_db_path(&root_path).to_string_lossy().to_string();
    let Some((conn, db_path)) = open_snapshot_connection(&root_path)? else {
        return Ok(GateSnapshot { db_path: db_path_string, exists: false, tasks: Vec::new() });
    };

    Ok(GateSnapshot { db_path, exists: true, tasks: load_tasks(&conn, &root_path)? })
}

#[tauri::command]
pub fn read_gate_task_markdown(path: String) -> Result<String, String> {
    let task_path = PathBuf::from(path);
    if !task_path.is_absolute() { return Err("Gate task path must be absolute".to_string()); }
    fs::read_to_string(&task_path).map_err(|e| format!("Failed to read {}: {e}", task_path.display()))
}

#[tauri::command]
pub fn start_gate_watcher(
    root: String,
    app: AppHandle,
    watcher_state: State<'_, GateWatcherManager>,
) -> Result<(), String> {
    let root_path = PathBuf::from(&root);
    if !root_path.is_absolute() { return Err("Gate root must be an absolute path".to_string()); }

    let watch_path = if gate_dir(&root_path).exists() { gate_dir(&root_path) } else { root_path.clone() };
    let (tx, rx) = mpsc::channel::<Event>();
    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let mut watcher = RecommendedWatcher::new(
        move |result| {
            if let Ok(event) = result { let _ = tx.send(event); }
        },
        Config::default(),
    ).map_err(|e| format!("Failed to create Gate watcher: {e}"))?;

    watcher
        .watch(&watch_path, RecursiveMode::NonRecursive)
        .map_err(|e| format!("Failed to watch {}: {e}", watch_path.display()))?;

    let join_handle = spawn_watch_thread(app, root, rx, stop_rx);
    watcher_state.replace(RunningGateWatcher { watcher, stop_tx, join_handle });
    Ok(())
}

#[tauri::command]
pub fn stop_gate_watcher(watcher_state: State<'_, GateWatcherManager>) {
    watcher_state.stop();
}
