use super::support::now_iso8601_roundtrip;
use crate::config::Config;
use crate::db;
use crate::error::Result;
use crate::markdown::{read_markdown, write_markdown};
use chrono::{DateTime, Utc};
use serde_yaml::Value as YamlValue;
use std::fs;
use std::time::{Duration, UNIX_EPOCH};

pub fn cleanup_abandoned_sessions(config: &Config) -> Result<usize> {
    // T-11.2: RTM FR-15
    let abandonment_minutes = std::env::var("MAENIFOLD_SESSION_ABANDON_MINUTES")
        .ok()
        .and_then(|value| value.trim().parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(30);
    let cleanup_started_at = now_iso8601_roundtrip();
    let cleanup_display_time = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let connection = db::open_connection(&config.database_path)?;
    let mut statement = connection.prepare(
        "SELECT file_path, last_indexed \
         FROM file_content \
         WHERE file_path LIKE 'memory://thinking/%' \
           AND status = 'active' \
           AND datetime(last_indexed) < datetime(?1, '-' || ?2 || ' minutes')",
    )?;
    let candidates = statement
        .query_map(
            rusqlite::params![cleanup_started_at, abandonment_minutes],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let mut abandoned_count = 0usize;
    for (file_uri, last_indexed) in candidates {
        match abandon_session_candidate(
            &connection,
            config,
            &file_uri,
            &last_indexed,
            abandonment_minutes,
            &cleanup_started_at,
            &cleanup_display_time,
        ) {
            Ok(true) => abandoned_count += 1,
            Ok(false) => {}
            Err(error) => {
                eprintln!("Warning: failed to abandon session candidate {file_uri}: {error}");
            }
        }
    }

    Ok(abandoned_count)
}

#[allow(clippy::too_many_arguments)]
fn abandon_session_candidate(
    connection: &rusqlite::Connection,
    config: &Config,
    file_uri: &str,
    last_indexed: &str,
    abandonment_minutes: i64,
    cleanup_started_at: &str,
    cleanup_display_time: &str,
) -> Result<bool> {
    // T-4.6: RTM FR-4.2 - explicit post-index filesystem touches override stale DB timestamps
    // without letting freshly-created test/session fixtures suppress legitimate stale cleanup.
    let path = crate::markdown::uri_to_path(file_uri, &config.memory_path)?;
    if let Ok(metadata) = fs::metadata(&path)
        && should_preserve_recent_filesystem_edit(&metadata, abandonment_minutes)
    {
        return Ok(false);
    }

    let (frontmatter, content, _) = read_markdown(file_uri, config)?;
    let mut frontmatter = frontmatter.unwrap_or_default();

    // Pipeline-owned workflow sessions have their own lifecycle management.
    // The pipeline engine controls state transitions with guards (e.g. must be Active
    // to transition to Abandoned). Session cleanup must not bypass those guards.
    if frontmatter.contains_key("pipelineId") {
        return Ok(false);
    }

    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("abandoned".to_string()),
    );

    let inactivity_minutes = parse_indexed_timestamp(last_indexed)
        .map(|indexed_at| {
            (Utc::now() - indexed_at)
                .num_minutes()
                .max(abandonment_minutes)
        })
        .unwrap_or(abandonment_minutes);
    let updated_content =
        append_abandoned_notice(&content, inactivity_minutes, cleanup_display_time);
    write_markdown(file_uri, Some(&frontmatter), &updated_content, config)?;

    let metadata = fs::metadata(&path)?;
    let (updated_frontmatter, updated_body, checksum) = read_markdown(file_uri, config)?;
    let title = updated_frontmatter
        .as_ref()
        .and_then(|values| values.get("title"))
        .and_then(YamlValue::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| {
            file_uri
                .trim_start_matches("memory://")
                .split('/')
                .next_back()
                .unwrap_or_default()
                .to_string()
        });

    connection.execute(
        "UPDATE file_content \
         SET title = ?2, content = ?3, last_indexed = ?4, status = ?5, file_md5 = ?6, file_size = ?7 \
         WHERE file_path = ?1",
        rusqlite::params![
            file_uri,
            title,
            updated_body,
            cleanup_started_at,
            "abandoned",
            checksum,
            metadata.len() as i64
        ],
    )?;

    Ok(true)
}

fn should_preserve_recent_filesystem_edit(
    metadata: &fs::Metadata,
    abandonment_minutes: i64,
) -> bool {
    // T-0.3: RTM NFR-4 - keep session cleanup deterministic across fresh fixture writes and
    // explicit filesystem mtime touches.
    let Ok(modified_at) = metadata.modified() else {
        return false;
    };
    let modified_at: DateTime<Utc> = modified_at.into();
    let abandonment_cutoff = Utc::now() - chrono::Duration::minutes(abandonment_minutes);
    if modified_at < abandonment_cutoff {
        return false;
    }

    metadata_change_lags_mtime(metadata, modified_at)
}

#[cfg(unix)]
fn metadata_change_lags_mtime(metadata: &fs::Metadata, modified_at: DateTime<Utc>) -> bool {
    use std::os::unix::fs::MetadataExt;

    if metadata.ctime().is_negative() {
        return false;
    }
    let changed_duration = Duration::new(metadata.ctime() as u64, metadata.ctime_nsec() as u32);
    let changed_at: DateTime<Utc> = (UNIX_EPOCH + changed_duration).into();
    changed_at - modified_at > chrono::Duration::milliseconds(1)
}

#[cfg(not(unix))]
fn metadata_change_lags_mtime(_metadata: &fs::Metadata, _modified_at: DateTime<Utc>) -> bool {
    true
}

fn append_abandoned_notice(
    content: &str,
    inactivity_minutes: i64,
    cleanup_display_time: &str,
) -> String {
    if content.contains("## Session Abandoned") {
        return content.to_string();
    }

    let trimmed = content.trim_end();
    if trimmed.is_empty() {
        format!(
            "## Session Abandoned\n\n⚠️ Session marked as abandoned due to {inactivity_minutes} minutes of inactivity\n*{cleanup_display_time}*\n"
        )
    } else {
        format!(
            "{trimmed}\n\n## Session Abandoned\n\n⚠️ Session marked as abandoned due to {inactivity_minutes} minutes of inactivity\n*{cleanup_display_time}*\n"
        )
    }
}

fn parse_indexed_timestamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|parsed| parsed.with_timezone(&Utc))
}
