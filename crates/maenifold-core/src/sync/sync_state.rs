use super::file_index::parse_indexed_timestamp;
use crate::error::Result;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use std::fs;
use std::path::Path;
use uuid::Uuid;

pub(crate) const COMMUNITY_PENDING_COUNT_KEY: &str = "pending_change_count";
pub(crate) const COMMUNITY_LAST_CHANGE_KEY: &str = "last_change_at";
pub(crate) const COMMUNITY_LAST_RECOMPUTE_KEY: &str = "last_community_recompute_at";

#[derive(Clone)]
pub(crate) struct SyncSummary {
    pub(crate) status: &'static str,
    pub(crate) files_processed: i64,
    pub(crate) concept_mentions: i64,
    pub(crate) concepts_filtered: i64,
    pub(crate) concept_relations: i64,
    pub(crate) abandoned_sessions: i64,
    pub(crate) communities_detected: i64,
    pub(crate) file_errors: Vec<(String, String)>,
    pub(crate) vector_warning_code: Option<&'static str>,
    // T-21.7: RTM FR-21.7 phase timing telemetry for [[sync]] markdown output.
    pub(crate) files_changed: i64,
    pub(crate) file_processing_secs: f64,
    pub(crate) graph_rebuild_secs: f64,
    pub(crate) embeddings_secs: f64,
    pub(crate) community_secs: f64,
    pub(crate) total_secs: f64,
}

pub(crate) fn ensure_incremental_sync_state_table(connection: &Connection) -> Result<()> {
    connection.execute(
        "CREATE TABLE IF NOT EXISTS incremental_sync_state (
             key TEXT PRIMARY KEY,
             value TEXT NOT NULL
         )",
        [],
    )?;
    Ok(())
}

pub(crate) fn record_incremental_graph_change(
    connection: &Connection,
    changed_at: &str,
) -> Result<()> {
    let next_count = load_incremental_state_i64(connection, COMMUNITY_PENDING_COUNT_KEY)? + 1;
    set_incremental_state(
        connection,
        COMMUNITY_PENDING_COUNT_KEY,
        &next_count.to_string(),
    )?;
    set_incremental_state(connection, COMMUNITY_LAST_CHANGE_KEY, changed_at)?;
    Ok(())
}

pub(crate) fn reset_incremental_community_state(
    connection: &Connection,
    recomputed_at: &str,
) -> Result<()> {
    ensure_incremental_sync_state_table(connection)?;
    set_incremental_state(connection, COMMUNITY_PENDING_COUNT_KEY, "0")?;
    set_incremental_state(connection, COMMUNITY_LAST_CHANGE_KEY, recomputed_at)?;
    set_incremental_state(connection, COMMUNITY_LAST_RECOMPUTE_KEY, recomputed_at)?;
    Ok(())
}

pub(crate) fn load_incremental_state_i64(connection: &Connection, key: &str) -> Result<i64> {
    let value = connection
        .query_row(
            "SELECT value FROM incremental_sync_state WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    Ok(value.and_then(|raw| raw.parse::<i64>().ok()).unwrap_or(0))
}

pub(crate) fn load_incremental_state_timestamp(
    connection: &Connection,
    key: &str,
) -> Option<DateTime<Utc>> {
    let value = connection
        .query_row(
            "SELECT value FROM incremental_sync_state WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .ok()
        .flatten()?;
    parse_indexed_timestamp(&value)
}

pub(crate) fn community_recompute_file_threshold() -> usize {
    std::env::var("MAENIFOLD_COMMUNITY_RECOMPUTE_FILE_THRESHOLD")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(10)
}

pub(crate) fn community_recompute_time_threshold() -> chrono::Duration {
    let threshold_ms = std::env::var("MAENIFOLD_COMMUNITY_DEBOUNCE_MS")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(2_000);
    chrono::Duration::milliseconds(threshold_ms)
}

pub(crate) fn community_gamma() -> f64 {
    // T-12.4: RTM NFR-1 keeps community recompute config local to [[sync]] after the module split.
    std::env::var("MAENIFOLD_LOUVAIN_GAMMA")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(1.0)
}

pub(crate) fn sync_role() -> String {
    match std::env::var("MAENIFOLD_SYNC_ROLE") {
        Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
            "leader" => "leader".to_string(),
            "follower" => "follower".to_string(),
            _ => "auto".to_string(),
        },
        Err(_) => "auto".to_string(),
    }
}

pub(crate) fn generate_sync_holder_id() -> String {
    format!("rust-{}-{}", std::process::id(), Uuid::new_v4())
}

pub(crate) fn render_sync_markdown(summary: SyncSummary, vector_section: Option<&str>) -> String {
    let mut output = format!(
        "Sync complete:
- STATUS: {}
- {} FILES processed
- {} CONCEPT mentions found
- {} CONCEPT mentions filtered
- {} CONCEPT relations created
- {} ABANDONED sessions cleaned up
- {} COMMUNITIES detected (modularity 0.000)",
        summary.status,
        summary.files_processed,
        summary.concept_mentions,
        summary.concepts_filtered,
        summary.concept_relations,
        summary.abandoned_sessions,
        summary.communities_detected,
    );
    if let Some(section) = vector_section {
        output.push('\n');
        output.push_str(section);
    } else if let Some(warning_code) = summary.vector_warning_code {
        // T-21.3: RTM-150 — [[sync]] verification content is now supplied by [[sync/mod.rs]]
        // after querying live vec0 tables, so only explicit warning codes fall back here.
        output.push_str(&format!("\n- WARNING_CODE {warning_code}"));
    }
    if !summary.file_errors.is_empty() {
        output.push_str("\n\n## Errors");
        for (file_uri, error_message) in &summary.file_errors {
            output.push_str(&format!("\n- `{file_uri}`: {error_message}"));
        }
    }
    // T-21.7: RTM FR-21.7 append phase timing telemetry after the vector section.
    output.push_str(&format!(
        "\nTiming:\n- File processing: {:.3}s ({} changed)\n- Graph rebuild: {:.3}s\n- Embeddings: {:.3}s\n- Community detection: {:.3}s\n- Total: {:.3}s",
        summary.file_processing_secs,
        summary.files_changed,
        summary.graph_rebuild_secs,
        summary.embeddings_secs,
        summary.community_secs,
        summary.total_secs,
    ));
    output
}

pub(crate) fn load_help_doc(assets_path: &Path, tool_name: &str) -> Option<String> {
    fs::read_to_string(
        assets_path
            .join("usage")
            .join("tools")
            .join(format!("{}.md", tool_name.to_ascii_lowercase())),
    )
    .ok()
}

fn set_incremental_state(connection: &Connection, key: &str, value: &str) -> Result<()> {
    connection.execute(
        "INSERT INTO incremental_sync_state (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub(crate) fn current_iso_timestamp() -> String {
    let now = chrono::Utc::now();
    format!(
        "{}.{:07}+00:00",
        now.format("%Y-%m-%dT%H:%M:%S"),
        now.timestamp_subsec_nanos() / 100,
    )
}

#[cfg(test)]
mod tests {
    use super::{SyncSummary, render_sync_markdown};

    #[test]
    fn render_sync_markdown_includes_errors_section() {
        let output = render_sync_markdown(
            SyncSummary {
                status: "partial",
                files_processed: 2,
                concept_mentions: 1,
                concepts_filtered: 0,
                concept_relations: 1,
                abandoned_sessions: 0,
                communities_detected: 0,
                file_errors: vec![("memory://broken.md".to_string(), "bad file".to_string())],
                vector_warning_code: None,
                files_changed: 1,
                file_processing_secs: 0.1,
                graph_rebuild_secs: 0.2,
                embeddings_secs: 0.3,
                community_secs: 0.4,
                total_secs: 1.0,
            },
            None,
        );

        assert!(output.contains("## Errors"));
        assert!(output.contains("- `memory://broken.md`: bad file"));
        assert!(output.contains("- STATUS: partial"));
    }
}
