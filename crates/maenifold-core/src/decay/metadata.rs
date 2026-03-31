use crate::config::Config;
use crate::db::{self, GraphDatabase};
use crate::error::{MaenifoldError, Result};
use crate::markdown::{Frontmatter, uri_to_path};
use chrono::{DateTime, NaiveDateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde_yaml::Value as YamlValue;
use std::fs;
use std::path::{Path, PathBuf};

use super::diagnostics::emit_system_tools_diagnostic;
use super::{compute_decay_weight, emit_degraded_decay_diagnostic};

// T-12.5: RTM NFR-1 groups frontmatter/DB/disk decay metadata lookup behind one
// module so callers keep the same helpers after converting decay.rs into a directory module.
fn yaml_value_to_string(value: &YamlValue) -> Option<String> {
    match value {
        YamlValue::String(value) => Some(value.clone()),
        YamlValue::Number(value) => Some(value.to_string()),
        YamlValue::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn parse_frontmatter_map(full_text: &str) -> Result<(Option<Frontmatter>, String)> {
    if let Some(rest) = full_text.strip_prefix("---\n")
        && let Some(index) = rest.find("\n---\n")
    {
        let yaml_text = &rest[..index];
        let content = rest[index + 5..]
            .trim_start_matches(['\r', '\n'])
            .to_string();
        let frontmatter: Frontmatter = serde_yaml::from_str(yaml_text)?;
        return Ok((Some(frontmatter), content));
    }
    Ok((None, full_text.to_string()))
}

fn system_time_to_datetime(file_path: &Path) -> Result<DateTime<Utc>> {
    let metadata = fs::metadata(file_path)?;
    let system_time = metadata
        .created()
        .or_else(|_| metadata.modified())
        .map_err(MaenifoldError::from)?;
    Ok(DateTime::<Utc>::from(system_time))
}

pub(crate) fn parse_datetime(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .ok()
        .or_else(|| {
            NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
                .ok()
                .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
        })
}

pub(crate) fn read_created_date(file_path: &Path) -> Result<DateTime<Utc>> {
    match fs::read_to_string(file_path) {
        Ok(full_text) => {
            let (frontmatter, _) = match parse_frontmatter_map(&full_text) {
                Ok(parsed) => parsed,
                Err(error) => {
                    emit_system_tools_diagnostic(
                        "GetDecayMetadataForFile",
                        "Failed to parse frontmatter; falling back to file-system timestamps",
                        &file_path.to_string_lossy(),
                        &Config::current(),
                        &error.to_string(),
                    );
                    return system_time_to_datetime(file_path);
                }
            };
            if let Some(parsed) = frontmatter
                .and_then(|frontmatter| frontmatter.get("created").and_then(yaml_value_to_string))
                .and_then(|created| parse_datetime(&created))
            {
                return Ok(parsed);
            }
            Ok(system_time_to_datetime(file_path)?)
        }
        Err(_) => Ok(system_time_to_datetime(file_path)?),
    }
}

pub(crate) fn get_decay_weight_for_file(file_identifier: &str, config: &Config) -> f64 {
    let connection = GraphDatabase::initialize_database(config)
        .and_then(|_| db::open_connection(&config.database_path));

    match connection {
        Ok(connection) => {
            get_decay_weight_for_file_with_connection(file_identifier, &connection, config)
        }
        Err(_) => get_decay_weight_from_disk(file_identifier, config),
    }
}

pub(crate) fn get_decay_weight_for_file_with_connection(
    file_identifier: &str,
    connection: &Connection,
    config: &Config,
) -> f64 {
    let row = connection
        .query_row(
            "SELECT created, last_accessed, status FROM file_content WHERE file_path = ?1",
            params![file_identifier],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .optional();

    match row {
        Ok(Some((created, last_accessed, status))) => {
            if let Some(last_accessed) = last_accessed.as_deref().and_then(parse_datetime) {
                return compute_decay_weight(
                    file_identifier,
                    Some(&last_accessed.to_rfc3339()),
                    config,
                    status.as_deref(),
                );
            }

            if let Some(created_value) = created.as_deref() {
                if let Some(created) = parse_datetime(created_value) {
                    return compute_decay_weight(
                        file_identifier,
                        Some(&created.to_rfc3339()),
                        config,
                        status.as_deref(),
                    );
                }

                emit_degraded_decay_diagnostic(
                    "GetDecayWeightForFile(reuse-conn)",
                    "Malformed file_content.created timestamp; using current UTC as degraded fallback",
                    file_identifier,
                    config,
                );
                return get_decay_weight_from_disk_with_status(
                    file_identifier,
                    config,
                    status.as_deref(),
                );
            }

            get_decay_weight_from_disk_with_status(file_identifier, config, status.as_deref())
        }
        Ok(None) | Err(_) => get_decay_weight_from_disk(file_identifier, config),
    }
}

pub(crate) fn resolve_decay_file_path(file_identifier: &str, config: &Config) -> Result<PathBuf> {
    if file_identifier.starts_with("memory://") {
        uri_to_path(file_identifier, &config.memory_path)
    } else {
        Ok(PathBuf::from(file_identifier))
    }
}

pub(crate) fn read_created_date_for_identifier(
    file_identifier: &str,
    config: &Config,
) -> Result<DateTime<Utc>> {
    let path = resolve_decay_file_path(file_identifier, config)?;
    read_created_date(&path)
}

pub(crate) fn get_decay_weight_from_disk_with_status(
    file_identifier: &str,
    config: &Config,
    status: Option<&str>,
) -> f64 {
    match read_created_date_for_identifier(file_identifier, config) {
        Ok(created) => {
            compute_decay_weight(file_identifier, Some(&created.to_rfc3339()), config, status)
        }
        Err(_) => {
            emit_degraded_decay_diagnostic(
                "GetDecayWeightFromDisk",
                "Falling back to neutral decay weight",
                file_identifier,
                config,
            );
            1.0
        }
    }
}

pub(crate) fn get_decay_weight_from_disk(file_identifier: &str, config: &Config) -> f64 {
    get_decay_weight_from_disk_with_status(file_identifier, config, None)
}
