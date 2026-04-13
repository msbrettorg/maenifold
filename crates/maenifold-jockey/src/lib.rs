pub mod acp;
mod assistant;
pub mod chat;
pub mod commands;
pub mod db;
pub mod emitter;
pub mod error;
pub mod fs_context;
pub mod jockey_mcp;
pub mod parser;
pub mod runtime_kind;
pub mod types;

use dashmap::DashMap;
use rusqlite::OptionalExtension;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

pub use assistant::refresh_assistant_catalog;
use crate::db::context::shared_key;
use crate::db::{init_db, seed_default_dynamic_catalog};
use crate::db::pool::DbPool;
pub use crate::types::*;

pub(crate) fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub(crate) fn default_chat_cwd() -> String {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| {
            std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "/tmp".to_string())
        })
}

pub(crate) fn abs_cwd(path: &str) -> String {
    let p = std::path::Path::new(path);
    let abs = if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::path::PathBuf::from(default_chat_cwd()).join(p)
    };
    abs.canonicalize()
        .unwrap_or(abs)
        .to_string_lossy()
        .to_string()
}

pub(crate) fn resolve_chat_cwd() -> String {
    abs_cwd(&default_chat_cwd())
}

pub(crate) fn clip_text(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }
    input.chars().take(max_chars).collect::<String>()
}

pub(crate) fn build_jockey_tool_prompt() -> &'static str {
    "You are Jockey assistant. Answer the user's question directly and concisely.\n\
IMPORTANT: Do NOT use any tools, read files, run commands, or explore the filesystem.\n\
\n\
App commands (prefix /app_) — only suggest when the user explicitly asks for Jockey management:\n\
  /app_help | /app_assistant list | /app_assistant select <runtime>\n\
Do NOT suggest role, model, or MCP commands — those are managed via the UI sidebar."
}

/// Type alias for the event broadcast sender used throughout the crate.
pub type EventSender = tokio::sync::broadcast::Sender<JockeyEvent>;

/// Event emitted by the Jockey runtime to external consumers (UI, SSE, WebSocket).
#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum JockeyEvent {
    AcpDelta(serde_json::Value),
    AcpStream(serde_json::Value),
    AcpConnectionLost(serde_json::Value),
    AcpPrewarm(serde_json::Value),
    CommandApplied(serde_json::Value),
    WorkflowStateChanged(serde_json::Value),
    SessionUpdate(serde_json::Value),
    McpError(String),
}

/// Jockey runtime handle returned by [`start`].
pub struct JockeyRuntime {
    pub state: Arc<AppState>,
    pub events: tokio::sync::broadcast::Receiver<JockeyEvent>,
    pub mcp_port: Option<u16>,
}

/// Initialize the Jockey runtime: open DB, start MCP bridge, wire event channels.
pub async fn start(data_dir: PathBuf) -> Result<JockeyRuntime, String> {
    fs::create_dir_all(&data_dir).map_err(|e| format!("create data dir: {e}"))?;
    acp::set_config_root(data_dir.clone());

    let db_path = data_dir.join("jockey.sqlite3");
    const CONN_INIT_SQL: &str =
        "PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA foreign_keys = ON;";
    let db_pool = DbPool::new(db_path, 8, CONN_INIT_SQL)?;
    {
        let conn = db_pool.get()?;
        init_db(&conn)?;
    }
    let state = Arc::new(AppState {
        db: db_pool,
        shared_context: DashMap::new(),
        role_cache: Arc::new(DashMap::new()),
    });

    // Restore shared context from DB
    {
        let guard = state.db.get()?;
        let mut stmt = guard
            .prepare(
                "SELECT scope, key, value FROM shared_context_snapshots ORDER BY updated_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            let (scope, key, value) = row.map_err(|e| e.to_string())?;
            state.shared_context.insert(shared_key(&scope, &key), value);
        }
    }

    seed_default_dynamic_catalog(&state)?;

    let (event_tx, event_rx) = tokio::sync::broadcast::channel::<JockeyEvent>(256);

    // Start MCP bridge
    let bridge_state = state.clone();
    let bridge_tx = event_tx.clone();
    let mcp_port = match jockey_mcp::bridge::start_bridge(bridge_state).await {
        Ok((port, token)) => {
            eprintln!("[jockey-mcp] listening on 127.0.0.1:{port}");
            db::global_mcp::seed_builtin_jockey_mcp(&state, port, &token);
            Some(port)
        }
        Err(e) => {
            eprintln!("[jockey-mcp] failed to start: {e}");
            let _ = bridge_tx.send(JockeyEvent::McpError(e));
            None
        }
    };

    // Wire ACP death events
    let (death_tx, mut death_rx) =
        tokio::sync::mpsc::unbounded_channel::<acp::ConnectionDeathEvent>();
    acp::set_death_event_sender(death_tx);
    let death_event_tx = event_tx.clone();
    tokio::spawn(async move {
        while let Some(event) = death_rx.recv().await {
            let _ = death_event_tx.send(JockeyEvent::AcpConnectionLost(
                serde_json::to_value(&event).unwrap_or_default(),
            ));
        }
    });

    // Wire ACP prewarm events
    let (prewarm_tx, mut prewarm_rx) =
        tokio::sync::mpsc::unbounded_channel::<acp::PrewarmEvent>();
    acp::set_prewarm_event_sender(prewarm_tx);
    let prewarm_event_tx = event_tx.clone();
    tokio::spawn(async move {
        while let Some(event) = prewarm_rx.recv().await {
            let _ = prewarm_event_tx.send(JockeyEvent::AcpPrewarm(
                serde_json::to_value(&event).unwrap_or_default(),
            ));
        }
    });

    Ok(JockeyRuntime {
        state,
        events: event_rx,
        mcp_port,
    })
}

/// Shut down the ACP worker and all active connections.
pub async fn shutdown() {
    acp::shutdown().await;
}
