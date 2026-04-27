//! Executor for parsed [`TeamsCommand`] values.
//!
//! Produces the reply body string; the caller (supervisor) is responsible
//! for actually sending the reply via [`super::reply::send_reply`]. Mirrors
//! the per-verb formatters in
//! `agency-cowork/skills/teams/scripts/monitor/message_handler.py` L950-1240.
#![allow(dead_code)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use tokio::sync::Mutex;

use super::commands::{DispatchFilter, TeamsCommand};
use super::dispatch_store::{DispatchRecord, DispatchStatus, DispatchStore};

/// Execute a command and return the reply body text.
/// Ref: `message_handler.py` L950-1240.
pub async fn execute_command(
    cmd: &TeamsCommand,
    paused: &AtomicBool,
    store: &Arc<Mutex<DispatchStore>>,
    monitored_conversations: &[String],
    start_time: Instant,
) -> String {
    match cmd {
        TeamsCommand::Status => format_status(paused, store, start_time).await,
        TeamsCommand::Stop => {
            paused.store(true, Ordering::Release);
            super::runtime::set_teams_paused(true);
            "⏸️ Monitor paused. Use !resume to unpause.".into()
        }
        TeamsCommand::Resume => {
            paused.store(false, Ordering::Release);
            super::runtime::set_teams_paused(false);
            "▶️ Monitor resumed.".into()
        }
        TeamsCommand::Cancel { arg } => format_cancel(arg.as_deref()),
        TeamsCommand::Dispatches { filter } => format_dispatches(store, *filter).await,
        TeamsCommand::List => format_list(monitored_conversations),
        TeamsCommand::Help => format_help(),
    }
}

/// Ref: `message_handler.py` L1126-L1142 (`_cmd_status`).
async fn format_status(
    paused: &AtomicBool,
    store: &Arc<Mutex<DispatchStore>>,
    start_time: Instant,
) -> String {
    let uptime = start_time.elapsed().as_secs();
    let hours = uptime / 3600;
    let minutes = (uptime % 3600) / 60;
    let seconds = uptime % 60;

    let (total, active) = {
        let guard = store.lock().await;
        let mut total = 0usize;
        let mut active = 0usize;
        for rec in guard.records() {
            total += 1;
            if matches!(
                rec.status,
                DispatchStatus::Pending | DispatchStatus::InProgress
            ) {
                active += 1;
            }
        }
        (total, active)
    };

    let pause_state = if paused.load(Ordering::Acquire) {
        "paused"
    } else {
        "running"
    };

    format!(
        "**Monitor Service Status**\n\n\
         - **Uptime:** {hours}h {minutes}m {seconds}s\n\
         - **State:** {pause_state}\n\
         - **Dispatches (total):** {total}\n\
         - **Active:** {active}"
    )
}

/// Cancel-by-id integration requires the live dispatch task handle owned by
/// the supervisor; surface a placeholder until that wiring lands.
fn format_cancel(_arg: Option<&str>) -> String {
    "Cancel not yet wired to active dispatches.".into()
}

/// Ref: `message_handler.py` L1252-L1279 (`_cmd_dispatches`). Truncates to
/// the most recent 10 entries to keep replies under Teams chunking limits.
async fn format_dispatches(store: &Arc<Mutex<DispatchStore>>, filter: DispatchFilter) -> String {
    let title = match filter {
        DispatchFilter::Pending => "Pending dispatches",
        DispatchFilter::Incomplete => "Incomplete dispatches",
        DispatchFilter::Recent => "Recent dispatches",
    };

    let records: Vec<DispatchRecord> = {
        let guard = store.lock().await;
        let mut v: Vec<DispatchRecord> = guard
            .records()
            .filter(|r| matches_filter(r, filter))
            .cloned()
            .collect();
        // Most recent first by enqueue time.
        v.sort_by(|a, b| {
            b.enqueued_at
                .partial_cmp(&a.enqueued_at)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        v.truncate(10);
        v
    };

    if records.is_empty() {
        return format!("No {} found.", title.to_lowercase());
    }

    let mut out = format!("**{title} (showing up to 10):**\n");
    for (i, r) in records.iter().enumerate() {
        let prompt: String = r.prompt.chars().take(80).collect();
        out.push_str(&format!(
            "\n{}. `{}` [{:?}] {} — {}",
            i + 1,
            r.id,
            r.status,
            r.sender_name,
            prompt,
        ));
    }
    out
}

fn matches_filter(rec: &DispatchRecord, filter: DispatchFilter) -> bool {
    match filter {
        DispatchFilter::Pending => rec.status == DispatchStatus::Pending,
        DispatchFilter::Incomplete => matches!(
            rec.status,
            DispatchStatus::Pending | DispatchStatus::InProgress
        ),
        DispatchFilter::Recent => true,
    }
}

/// Ref: `message_handler.py` L1119-L1124 (`_cmd_list`).
fn format_list(monitored: &[String]) -> String {
    if monitored.is_empty() {
        return "No conversations are being monitored.".into();
    }
    let mut out = String::from("**Monitored conversations:**");
    for c in monitored {
        out.push_str(&format!("\n- {c}"));
    }
    out
}

/// Help text — covers the verbs accepted by [`super::commands::parse_command`].
fn format_help() -> String {
    "**Available commands:**\n\
     - `!status` — service uptime, pause state, dispatch counts\n\
     - `!stop` / `!pause` — pause dispatch (queues fill but don't run)\n\
     - `!resume` / `!start` — resume dispatch\n\
     - `!cancel [id]` — cancel a dispatch (placeholder)\n\
     - `!dispatches [pending|incomplete]` — list recent / filtered dispatches\n\
     - `!list` / `!conversations` — list monitored conversations\n\
     - `!help` / `!?` — show this help"
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn fresh_store() -> Arc<Mutex<DispatchStore>> {
        let dir = tempdir().unwrap();
        let path: PathBuf = dir.path().join("dispatch.json");
        // Leak the tempdir so the file outlives this helper for the duration of the test.
        std::mem::forget(dir);
        Arc::new(Mutex::new(DispatchStore::open(path).unwrap()))
    }

    #[tokio::test]
    async fn stop_sets_paused() {
        let paused = AtomicBool::new(false);
        let store = fresh_store();
        let reply =
            execute_command(&TeamsCommand::Stop, &paused, &store, &[], Instant::now()).await;
        assert!(reply.contains("paused"));
        assert!(paused.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn resume_clears_paused() {
        let paused = AtomicBool::new(true);
        let store = fresh_store();
        execute_command(&TeamsCommand::Resume, &paused, &store, &[], Instant::now()).await;
        assert!(!paused.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn status_includes_uptime_and_state() {
        let paused = AtomicBool::new(false);
        let store = fresh_store();
        let reply =
            execute_command(&TeamsCommand::Status, &paused, &store, &[], Instant::now()).await;
        assert!(reply.contains("Uptime"));
        assert!(reply.contains("running"));
    }

    #[tokio::test]
    async fn list_empty() {
        let paused = AtomicBool::new(false);
        let store = fresh_store();
        let reply =
            execute_command(&TeamsCommand::List, &paused, &store, &[], Instant::now()).await;
        assert!(reply.contains("No conversations"));
    }

    #[tokio::test]
    async fn list_renders_entries() {
        let paused = AtomicBool::new(false);
        let store = fresh_store();
        let convs = vec!["chat-1".to_string(), "chat-2".to_string()];
        let reply =
            execute_command(&TeamsCommand::List, &paused, &store, &convs, Instant::now()).await;
        assert!(reply.contains("chat-1"));
        assert!(reply.contains("chat-2"));
    }

    #[tokio::test]
    async fn dispatches_empty_message() {
        let paused = AtomicBool::new(false);
        let store = fresh_store();
        let reply = execute_command(
            &TeamsCommand::Dispatches {
                filter: DispatchFilter::Recent,
            },
            &paused,
            &store,
            &[],
            Instant::now(),
        )
        .await;
        assert!(reply.to_lowercase().contains("no recent dispatches"));
    }

    #[tokio::test]
    async fn help_lists_verbs() {
        let paused = AtomicBool::new(false);
        let store = fresh_store();
        let reply =
            execute_command(&TeamsCommand::Help, &paused, &store, &[], Instant::now()).await;
        for verb in [
            "!status",
            "!stop",
            "!resume",
            "!cancel",
            "!dispatches",
            "!list",
            "!help",
        ] {
            assert!(reply.contains(verb), "missing {verb}");
        }
    }
}
