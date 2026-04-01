use super::context::{ToolContext, load_help_doc};
use super::recent_activity_format::format_recent_activity_report;
use crate::config::Config;
use crate::db;
use crate::error::{MaenifoldError, Result};
use crate::markdown::read_markdown;
use crate::util::json::{OutputFormat, success};
use chrono::{TimeDelta, Utc};
use rusqlite::{Connection, params};
use serde_json::Value;
use serde_yaml::Value as YamlValue;

pub fn recent_activity(context: &ToolContext, payload: &Value) -> Result<String> {
    let learn = payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if learn {
        let help = load_help_doc(&context.config.assets_path, "RecentActivity")
            .unwrap_or_else(|| "ERROR: Help file not found for RecentActivity".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }

    // T-2.1: RTM FR-4 / RTM-026 default limit handling for RecentActivity.
    let limit = payload.get("limit").and_then(Value::as_i64).unwrap_or(10);
    if limit <= 0 {
        return Err(MaenifoldError::InvalidArgument(
            "limit must be a positive integer".to_string(),
        ));
    }
    // T-2.6: RTM FR-4 / RTM-026 clamp RecentActivity limit to prevent unbounded queries.
    let limit = limit.min(1_000);

    // T-2.1: RTM FR-4 / RTM-021-RTM-024 filter and timespan validation.
    let filter = parse_recent_activity_filter(payload.get("filter").and_then(Value::as_str))?;
    let include_content = payload
        .get("includeContent")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let timespan = payload
        .get("timespan")
        .and_then(Value::as_str)
        .map(parse_recent_activity_timespan)
        .transpose()?;

    let rows =
        collect_recent_activity_records_with_filter(&context.config, timespan, filter, limit)?;
    let report = format_recent_activity_report(rows, include_content);

    Ok(match context.output_format {
        OutputFormat::Json => success(report),
        OutputFormat::Markdown => report,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecentActivityRecord {
    pub file_path: String,
    pub title: String,
    pub last_indexed: String,
    pub content: String,
    pub status: String,
    pub first: Option<String>,
    pub last: Option<String>,
}

#[derive(Clone, Copy)]
enum RecentActivityFilter {
    All,
    Thinking,
    Memory,
    Assumptions,
}

impl RecentActivityFilter {
    fn sql_clause(self) -> &'static str {
        match self {
            Self::All => "1=1",
            Self::Thinking => "file_path LIKE 'memory://thinking/%'",
            Self::Memory => {
                "file_path NOT LIKE 'memory://thinking/%' AND file_path NOT LIKE 'memory://chat/%'"
            }
            Self::Assumptions => "file_path LIKE 'memory://chat/%'",
        }
    }
}

fn parse_recent_activity_filter(filter: Option<&str>) -> Result<RecentActivityFilter> {
    match filter
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
    {
        None => Ok(RecentActivityFilter::All),
        Some(value) if value == "all" => Ok(RecentActivityFilter::All),
        Some(value) if value == "thinking" => Ok(RecentActivityFilter::Thinking),
        Some(value) if value == "memory" => Ok(RecentActivityFilter::Memory),
        Some(value) if value == "assumptions" => Ok(RecentActivityFilter::Assumptions),
        Some(value) => Err(MaenifoldError::InvalidArgument(format!(
            "Invalid filter '{value}'. Valid values: thinking, memory, assumptions, all"
        ))),
    }
}

pub fn parse_recent_activity_timespan(input: &str) -> Result<TimeDelta> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(MaenifoldError::InvalidArgument(
            "timespan must not be empty".to_string(),
        ));
    }
    if trimmed.starts_with('-') {
        return Err(MaenifoldError::InvalidArgument(
            "timespan must be positive".to_string(),
        ));
    }

    // T-2.1: RTM FR-4 / RTM-024 parse RecentActivity timespan in C#-style form.
    // Compatibility note: the shipped docs/examples use values like "24.00:00:00" for one day,
    // so values >= 24 before the dot are treated as total hours rather than calendar days.
    let (day_or_hour_prefix, time_part) = match trimmed.split_once('.') {
        Some((prefix, time_part)) => (prefix, time_part),
        None => ("0", trimmed),
    };
    let day_or_hour_prefix: i64 = day_or_hour_prefix
        .parse()
        .map_err(|_| invalid_timespan(trimmed))?;
    let mut time_segments = time_part.split(':');
    let hours: i64 = time_segments
        .next()
        .ok_or_else(|| invalid_timespan(trimmed))?
        .parse()
        .map_err(|_| invalid_timespan(trimmed))?;
    let minutes: i64 = time_segments
        .next()
        .ok_or_else(|| invalid_timespan(trimmed))?
        .parse()
        .map_err(|_| invalid_timespan(trimmed))?;
    let seconds: i64 = time_segments
        .next()
        .ok_or_else(|| invalid_timespan(trimmed))?
        .parse()
        .map_err(|_| invalid_timespan(trimmed))?;
    if time_segments.next().is_some() || hours < 0 || minutes < 0 || seconds < 0 {
        return Err(invalid_timespan(trimmed));
    }
    if minutes >= 60 || seconds >= 60 {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Invalid timespan '{trimmed}'. Minutes and seconds must be less than 60"
        )));
    }

    let (days, total_hours) = if trimmed.contains('.') && day_or_hour_prefix >= 24 {
        (0, day_or_hour_prefix + hours)
    } else {
        (day_or_hour_prefix, hours)
    };

    TimeDelta::try_days(days)
        .and_then(|delta| delta.checked_add(&TimeDelta::try_hours(total_hours)?))
        .and_then(|delta| delta.checked_add(&TimeDelta::try_minutes(minutes)?))
        .and_then(|delta| delta.checked_add(&TimeDelta::try_seconds(seconds)?))
        .ok_or_else(|| {
            MaenifoldError::InvalidArgument(format!(
                "Invalid timespan '{trimmed}'. Value is out of range"
            ))
        })
}

fn invalid_timespan(trimmed: &str) -> MaenifoldError {
    MaenifoldError::InvalidArgument(format!(
        "Invalid timespan '{trimmed}'. Expected format days.HH:MM:SS"
    ))
}

pub fn collect_recent_activity_records(
    config: &Config,
    timespan: Option<TimeDelta>,
    filter: Option<&str>,
    limit: i64,
) -> Result<Vec<RecentActivityRecord>> {
    let filter = parse_recent_activity_filter(filter)?;
    collect_recent_activity_records_with_filter(config, timespan, filter, limit)
}

// T-2.1: RTM FR-4 / RTM-020-RTM-029 exposes RecentActivity query behavior for pipeline tests.
fn collect_recent_activity_records_with_filter(
    config: &Config,
    timespan: Option<TimeDelta>,
    filter: RecentActivityFilter,
    limit: i64,
) -> Result<Vec<RecentActivityRecord>> {
    let connection = db::open_connection(&config.database_path)?;
    if !recent_activity_table_exists(&connection)? {
        return Err(MaenifoldError::Message(
            "No database found. Run the `Sync` command first to index memory files.".to_string(),
        ));
    }

    let filter_clause = filter.sql_clause();
    let mut sql = format!(
        "SELECT file_path, COALESCE(title, ''), COALESCE(last_indexed, ''), COALESCE(content, ''), COALESCE(status, '') \
         FROM file_content WHERE {filter_clause}"
    );

    let cutoff = timespan.map(|span| (Utc::now() - span).format("%Y-%m-%d %H:%M:%S").to_string());
    if cutoff.is_some() {
        sql.push_str(" AND datetime(last_indexed) >= datetime(?)");
    }
    sql.push_str(" ORDER BY datetime(last_indexed) DESC LIMIT ?");

    let mut statement = connection.prepare(&sql)?;
    let rows = if let Some(cutoff) = cutoff.as_deref() {
        statement.query_map(params![cutoff, limit], map_recent_activity_row)?
    } else {
        statement.query_map(params![limit], map_recent_activity_row)?
    };

    let mut records = rows
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(MaenifoldError::from)?;

    for record in &mut records {
        populate_recent_activity_summaries(record, config);
    }

    Ok(records)
}

fn recent_activity_table_exists(connection: &Connection) -> Result<bool> {
    Ok(connection.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='file_content'",
        [],
        |row| row.get::<_, i64>(0),
    )? > 0)
}

fn map_recent_activity_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RecentActivityRecord> {
    Ok(RecentActivityRecord {
        file_path: row.get(0)?,
        title: row.get(1)?,
        last_indexed: row.get(2)?,
        content: row.get(3)?,
        status: row.get(4)?,
        first: None,
        last: None,
    })
}

fn populate_recent_activity_summaries(record: &mut RecentActivityRecord, config: &Config) {
    let (content_first, content_last) = parse_frontmatter_summaries(&record.content);
    record.first = content_first;
    record.last = content_last;

    if record.first.is_some() && record.last.is_some() {
        return;
    }

    let Ok((frontmatter, _, _)) = read_markdown(&record.file_path, config) else {
        return;
    };
    let Some(frontmatter) = frontmatter else {
        return;
    };

    if record.first.is_none() {
        record.first = frontmatter_map_string(&frontmatter, "first");
    }
    if record.last.is_none() {
        record.last = frontmatter_map_string(&frontmatter, "last");
    }
}

fn parse_frontmatter_summaries(content: &str) -> (Option<String>, Option<String>) {
    let Some((yaml_text, _body)) = split_embedded_frontmatter(content) else {
        return (None, None);
    };
    let Ok(frontmatter) = serde_yaml::from_str::<YamlValue>(yaml_text) else {
        return (None, None);
    };

    (
        frontmatter_yaml_string(&frontmatter, "first"),
        frontmatter_yaml_string(&frontmatter, "last"),
    )
}

fn split_embedded_frontmatter(content: &str) -> Option<(&str, &str)> {
    let yaml_start = if content.starts_with("---\n") {
        4
    } else if content.starts_with("---\r\n") {
        5
    } else {
        return None;
    };

    let mut cursor = yaml_start;
    while cursor <= content.len() {
        let remaining = &content[cursor..];
        match remaining.find('\n') {
            Some(line_end) => {
                let line = remaining[..line_end].trim_end_matches('\r');
                if line == "---" {
                    let yaml_text = &content[yaml_start..cursor];
                    let body = &remaining[line_end + 1..];
                    return Some((yaml_text, body));
                }
                cursor += line_end + 1;
            }
            None => {
                if remaining.trim_end_matches('\r') == "---" {
                    let yaml_text = &content[yaml_start..cursor];
                    return Some((yaml_text, ""));
                }
                return None;
            }
        }
    }

    None
}

fn normalize_frontmatter_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn frontmatter_map_string(
    frontmatter: &std::collections::BTreeMap<String, YamlValue>,
    key: &str,
) -> Option<String> {
    normalize_frontmatter_string(frontmatter.get(key).and_then(YamlValue::as_str))
}

fn frontmatter_yaml_string(frontmatter: &YamlValue, key: &str) -> Option<String> {
    normalize_frontmatter_string(frontmatter.get(key).and_then(YamlValue::as_str))
}
