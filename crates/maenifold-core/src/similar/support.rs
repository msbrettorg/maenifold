use crate::config::Config;
use crate::decay::compute_decay_weight_with_created;
use crate::error::Result;
use crate::tools::with_next_steps;
use rusqlite::Connection;
use std::fs;
use std::path::Path;

// T-13.5: RTM FR-8 — use centralized with_next_steps helper matching C# ToolResponse.WithNextSteps
pub(crate) fn with_similarity_next_steps(message: impl Into<String>) -> String {
    with_next_steps(&message.into(), &["BuildContext", "SearchMemories"])
}

pub(crate) fn concept_decay_weight(
    connection: &Connection,
    config: &Config,
    concept_name: &str,
) -> Result<f64> {
    let mut statement = connection.prepare(
        "SELECT cm.mention_count, fc.file_path, fc.created, fc.last_accessed, fc.status
         FROM concept_mentions cm
         LEFT JOIN file_content fc ON fc.file_path = cm.source_file
         WHERE cm.concept_name = ?1",
    )?;
    let rows = statement.query_map([concept_name], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
        ))
    })?;

    let mut total_mentions = 0.0;
    let mut total_weight = 0.0;
    for row in rows {
        let (mention_count, file_path, created, last_accessed, status) = row?;
        let mentions = mention_count.max(1) as f64;
        let file_weight = file_path
            .as_deref()
            .map(|path| {
                // T-14.1: RTM FR-21 keeps concept decay aligned with C#
                // fallback semantics when access metadata is absent.
                compute_decay_weight_with_created(
                    path,
                    last_accessed.as_deref(),
                    created.as_deref(),
                    config,
                    status.as_deref(),
                )
            })
            .unwrap_or(1.0);
        total_mentions += mentions;
        total_weight += mentions * file_weight;
    }

    if total_mentions <= 0.0 {
        return Ok(1.0);
    }

    Ok((total_weight / total_mentions).clamp(0.0, 1.0))
}

pub(crate) fn load_help_doc(assets_path: &Path, tool_name: &str) -> Option<String> {
    // T-12.3: RTM NFR-1 preserves the existing learn-mode help-doc lookup while splitting similar.rs.
    fs::read_to_string(
        assets_path
            .join("usage")
            .join("tools")
            .join(format!("{}.md", tool_name.to_ascii_lowercase())),
    )
    .ok()
}
