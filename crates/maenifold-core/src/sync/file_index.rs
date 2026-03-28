use super::sync_state::current_iso_timestamp;
use crate::error::Result;
use crate::markdown::{Frontmatter, WikiLinkFilter, extract_wikilinks};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub(crate) struct IndexedFileState {
    pub(crate) last_indexed: Option<String>,
    pub(crate) file_md5: Option<String>,
    pub(crate) has_mentions: bool,
}

pub(crate) fn load_indexed_file_state(
    connection: &Connection,
    file_uri: &str,
) -> Result<Option<IndexedFileState>> {
    Ok(connection
        .query_row(
            "SELECT \
                 last_indexed, \
                 file_md5, \
                 EXISTS(SELECT 1 FROM concept_mentions WHERE source_file = file_content.file_path) \
             FROM file_content \
             WHERE file_path = ?1",
            params![file_uri],
            |row| {
                Ok(IndexedFileState {
                    last_indexed: row.get(0)?,
                    file_md5: row.get(1)?,
                    // T-0.1: RTM NFR-3 - Sync must rebuild graph state when WriteMemory pre-indexed file_content rows lack concept_mentions.
                    has_mentions: row.get::<_, i64>(2)? != 0,
                })
            },
        )
        .optional()?)
}

pub(crate) fn should_bootstrap_graph_state(indexed_file: &IndexedFileState) -> bool {
    !indexed_file.has_mentions
        && indexed_file
            .last_indexed
            .as_deref()
            // T-0.1: RTM NFR-3 - WriteMemory pre-indexes file_content with second-precision UTC timestamps like 2026-03-09T17:38:24Z.
            .map(|value| value.len() == 20 && value.ends_with('Z') && !value.contains('+'))
            .unwrap_or(false)
}

pub(crate) fn delete_missing_files(
    connection: &Connection,
    existing_files: &HashSet<String>,
) -> Result<()> {
    let mut statement = connection.prepare("SELECT file_path FROM file_content")?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    let indexed_files = rows.collect::<std::result::Result<Vec<_>, _>>()?;

    for file_uri in indexed_files {
        if !existing_files.contains(&file_uri) {
            tx_delete_file_index(connection, &file_uri)?;
        }
    }

    Ok(())
}

pub(crate) fn load_file_concepts(
    connection: &Connection,
    file_uri: &str,
) -> Result<BTreeSet<String>> {
    let mut statement = connection.prepare(
        "SELECT concept_name FROM concept_mentions WHERE source_file = ?1 ORDER BY concept_name",
    )?;
    let rows = statement.query_map(params![file_uri], |row| row.get::<_, String>(0))?;
    Ok(rows.collect::<std::result::Result<BTreeSet<_>, _>>()?)
}

pub(crate) fn load_total_mentions_for_file(connection: &Connection, file_uri: &str) -> Result<i64> {
    Ok(connection.query_row(
        "SELECT COALESCE(SUM(mention_count), 0) FROM concept_mentions WHERE source_file = ?1",
        params![file_uri],
        |row| row.get(0),
    )?)
}

pub(crate) fn tx_delete_file_index(connection: &Connection, file_uri: &str) -> Result<()> {
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

pub(crate) fn should_skip_by_mtime(
    modified_at: Option<SystemTime>,
    last_indexed: Option<&str>,
) -> bool {
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

pub(crate) fn parse_indexed_timestamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|parsed| parsed.with_timezone(&Utc))
}

pub(crate) fn indexed_timestamp_for_metadata(metadata: &fs::Metadata) -> String {
    metadata
        .modified()
        .ok()
        .map(|modified| DateTime::<Utc>::from(modified).to_rfc3339())
        .unwrap_or_else(current_iso_timestamp)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn upsert_processed_file(
    connection: &Connection,
    uri: &str,
    frontmatter: &Option<Frontmatter>,
    content: &str,
    checksum: &str,
    file_size: i64,
    indexed_at: &str,
    filter: Option<&WikiLinkFilter>,
) -> Result<i64> {
    let title = frontmatter
        .as_ref()
        .and_then(|frontmatter| frontmatter.get("title"))
        .and_then(|value| value.as_str())
        // T-0.3b: RTM NFR-4
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
        "INSERT INTO file_content (file_path, title, content, summary, last_indexed, status, file_md5, file_size, created, last_accessed) \
         VALUES (?1, ?2, ?3, '', ?4, ?5, ?6, ?7, COALESCE(?8, ?4), COALESCE((SELECT last_accessed FROM file_content WHERE file_path = ?1), COALESCE(?8, ?4))) \
         ON CONFLICT(file_path) DO UPDATE SET title=excluded.title, content=excluded.content, summary=excluded.summary, last_indexed=excluded.last_indexed, status=excluded.status, file_md5=excluded.file_md5, file_size=excluded.file_size, created=COALESCE(file_content.created, excluded.created), last_accessed=COALESCE(file_content.last_accessed, excluded.last_accessed)",
        params![uri, title, content, indexed_at, status, checksum, file_size, created],
    )?;

    connection.execute(
        "DELETE FROM concept_mentions WHERE source_file = ?1",
        params![uri],
    )?;

    // T-22.2: RTM-151, RTM-152, RTM-155 — apply [[WikiLink Filter]] during sync concept extraction.
    let mut per_file = BTreeMap::<String, usize>::new();
    let mut filtered_count = 0i64;
    for concept in extract_wikilinks(content) {
        let normalized = concept.to_ascii_lowercase();
        if let Some(f) = filter
            && f.is_concept_blocked(&normalized)
        {
            filtered_count += 1;
            continue;
        }
        *per_file.entry(normalized).or_default() += 1;
    }

    for (concept, mention_count) in per_file {
        connection.execute(
            "INSERT OR IGNORE INTO concepts (concept_name, first_seen, occurrence_count) VALUES (?1, ?2, 0)",
            params![concept, indexed_at],
        )?;
        connection.execute(
            "INSERT INTO concept_mentions (concept_name, source_file, mention_count) VALUES (?1, ?2, ?3) \
             ON CONFLICT(concept_name, source_file) DO UPDATE SET mention_count = excluded.mention_count",
            params![concept, uri, mention_count as i64],
        )?;
    }

    Ok(filtered_count)
}
