use crate::assets::{AssetKind, AssetManager};
use crate::config::Config;
use crate::db;
use crate::error::{MaenifoldError, Result};
use crate::json::{OutputFormat, failure, success};
use crate::markdown::extract_h2_sections;
use chrono::{DateTime, TimeDelta, Utc};
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

pub struct ToolContext {
    pub config: Config,
    pub output_format: OutputFormat,
}

impl ToolContext {
    pub fn new(config: Config, output_format: OutputFormat) -> Self {
        Self {
            config,
            output_format,
        }
    }
}

pub fn list_assets(context: &ToolContext, payload: &Value) -> Result<String> {
    let asset_manager = AssetManager::new(&context.config);
    let kind = payload
        .get("type")
        .and_then(Value::as_str)
        .map(AssetKind::parse)
        .transpose()?;
    let value = asset_manager.list_assets(kind)?;
    Ok(match context.output_format {
        // T-1.6: RTM NFR-1 - CLI JSON mode wraps rendered text in the C# envelope.
        OutputFormat::Json => success(render(OutputFormat::Markdown, value)),
        OutputFormat::Markdown => render(OutputFormat::Markdown, value),
    })
}

pub fn read_mcp_resource(context: &ToolContext, payload: &Value) -> Result<String> {
    let uri = payload
        .get("uri")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("uri"))?;
    let asset_manager = AssetManager::new(&context.config);
    let value = asset_manager.read_resource(uri)?;
    Ok(match context.output_format {
        OutputFormat::Json => success(render(OutputFormat::Markdown, value)),
        OutputFormat::Markdown => render(OutputFormat::Markdown, value),
    })
}

pub fn get_config(context: &ToolContext, payload: &Value) -> Result<String> {
    // T-1.5: RTM FR-1 / FR-3 / FR-4 / FR-5 / FR-6 preserve C#-compatible learn mode for system diagnostics.
    let learn = payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if learn {
        let help = load_help_doc(&context.config.assets_path, "GetConfig")
            .unwrap_or_else(|| "ERROR: Help file not found for GetConfig".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }

    let rendered = render_config_markdown(context);
    Ok(match context.output_format {
        OutputFormat::Json => success(rendered),
        OutputFormat::Markdown => rendered,
    })
}

pub fn memory_status(context: &ToolContext, payload: &Value) -> Result<String> {
    // T-1.5: RTM FR-1 / FR-3 / FR-4 / FR-5 / FR-6 preserve C#-compatible learn mode for system diagnostics.
    let learn = payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if learn {
        let help = load_help_doc(&context.config.assets_path, "MemoryStatus")
            .unwrap_or_else(|| "ERROR: Help file not found for MemoryStatus".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }

    let file_count = walkdir::WalkDir::new(&context.config.memory_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count();
    let value = json!({
        "fileCount": file_count,
        "databasePath": context.config.database_path,
    });
    Ok(render(context.output_format, value))
}

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

pub fn not_yet_implemented(context: &ToolContext, tool_name: &str) -> String {
    match context.output_format {
        OutputFormat::Json => failure(
            "NOT_IMPLEMENTED",
            format!("Tool not yet implemented: {tool_name}"),
        ),
        OutputFormat::Markdown => format!("ERROR: Tool not yet implemented: {tool_name}"),
    }
}

fn load_help_doc(assets_path: &Path, tool_name: &str) -> Option<String> {
    fs::read_to_string(
        assets_path
            .join("usage")
            .join("tools")
            .join(format!("{}.md", tool_name.to_ascii_lowercase())),
    )
    .ok()
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecentActivityRecord {
    pub file_path: String,
    pub title: String,
    pub last_indexed: String,
    pub content: String,
    pub status: String,
}

#[derive(Clone, Copy)]
enum RecentActivityType {
    Sequential,
    Workflow,
    Thinking,
    Memory,
}

impl RecentActivityType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Sequential => "sequential",
            Self::Workflow => "workflow",
            Self::Thinking => "thinking",
            Self::Memory => "memory",
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

fn parse_recent_activity_timespan(input: &str) -> Result<TimeDelta> {
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
    let day_or_hour_prefix: i64 = day_or_hour_prefix.parse().map_err(|_| {
        MaenifoldError::InvalidArgument(format!(
            "Invalid timespan '{trimmed}'. Expected format days.HH:MM:SS"
        ))
    })?;
    let mut time_segments = time_part.split(':');
    let hours: i64 = time_segments
        .next()
        .ok_or_else(|| {
            MaenifoldError::InvalidArgument(format!(
                "Invalid timespan '{trimmed}'. Expected format days.HH:MM:SS"
            ))
        })?
        .parse()
        .map_err(|_| {
            MaenifoldError::InvalidArgument(format!(
                "Invalid timespan '{trimmed}'. Expected format days.HH:MM:SS"
            ))
        })?;
    let minutes: i64 = time_segments
        .next()
        .ok_or_else(|| {
            MaenifoldError::InvalidArgument(format!(
                "Invalid timespan '{trimmed}'. Expected format days.HH:MM:SS"
            ))
        })?
        .parse()
        .map_err(|_| {
            MaenifoldError::InvalidArgument(format!(
                "Invalid timespan '{trimmed}'. Expected format days.HH:MM:SS"
            ))
        })?;
    let seconds: i64 = time_segments
        .next()
        .ok_or_else(|| {
            MaenifoldError::InvalidArgument(format!(
                "Invalid timespan '{trimmed}'. Expected format days.HH:MM:SS"
            ))
        })?
        .parse()
        .map_err(|_| {
            MaenifoldError::InvalidArgument(format!(
                "Invalid timespan '{trimmed}'. Expected format days.HH:MM:SS"
            ))
        })?;
    if time_segments.next().is_some() || hours < 0 || minutes < 0 || seconds < 0 {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Invalid timespan '{trimmed}'. Expected format days.HH:MM:SS"
        )));
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

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(MaenifoldError::from)
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
    })
}

fn classify_recent_activity_type(file_path: &str) -> RecentActivityType {
    // T-2.1: RTM FR-4 / RTM-029 path classification for RecentActivity.
    if file_path.contains("/sequential/") {
        RecentActivityType::Sequential
    } else if file_path.contains("/workflow/") {
        RecentActivityType::Workflow
    } else if file_path.starts_with("memory://thinking/") {
        RecentActivityType::Thinking
    } else {
        RecentActivityType::Memory
    }
}

// T-2.1: RTM FR-4 / RTM-029 exposes path classification parity for RecentActivity pipeline tests.
pub fn recent_activity_determine_file_type(file_path: &str) -> &'static str {
    classify_recent_activity_type(file_path).as_str()
}

// T-2.3: RTM FR-4 exposes session-id validation used by pure date-detection regression tests.
pub fn is_valid_session_id_format(session_id: &str) -> bool {
    session_id
        .split('-')
        .nth(1)
        .filter(|segment| !segment.is_empty())
        .and_then(|segment| segment.parse::<i64>().ok())
        .is_some()
}

// T-2.1: RTM FR-4 / RTM-029 exposes session-id extraction parity for RecentActivity pipeline tests.
pub fn recent_activity_extract_session_id(file_path: &str) -> String {
    extract_recent_activity_session_id(file_path)
}

// T-2.1: RTM FR-4 / RTM-025 exposes formatter behavior for RecentActivity pipeline tests.
pub fn format_recent_activity_report(
    rows: Vec<RecentActivityRecord>,
    include_content: bool,
) -> String {
    let entries = rows
        .iter()
        .map(|row| {
            let file_type = classify_recent_activity_type(&row.file_path);
            let sections = extract_h2_sections(&row.content);
            format_recent_activity_entry(row, file_type, sections, include_content)
        })
        .collect::<Vec<_>>();

    if entries.is_empty() {
        "No recent activity found".to_string()
    } else {
        format!("# Recent Activity\n\n{}", entries.join("\n\n"))
    }
}

fn format_recent_activity_entry(
    row: &RecentActivityRecord,
    file_type: RecentActivityType,
    sections: Vec<(String, String)>,
    include_content: bool,
) -> String {
    let session_id = extract_recent_activity_session_id(&row.file_path);
    let modified = format_recent_activity_timestamp(&row.last_indexed);
    let (count_label, count_value) = count_recent_activity_sections(file_type, &sections);
    let status = if row.status.trim().is_empty() {
        "unknown"
    } else {
        row.status.trim()
    };

    let mut lines = vec![format!("**{session_id}** ({})", file_type.as_str())];
    lines.push(format!("  Modified: {modified}"));
    lines.push(format!("  {count_label}: {count_value}"));
    lines.push(format!("  Status: {status}"));

    if matches!(file_type, RecentActivityType::Memory) && !row.title.trim().is_empty() {
        lines.push(format!("  Title: {}", row.title.trim()));
    }

    if include_content {
        let (first_heading, first_content) = section_details(sections.first());
        let (last_heading, last_content) = section_details(sections.last());
        lines.push(format!("  First H2: {first_heading}"));
        lines.push(format!("  First H2 Content: {first_content}"));
        lines.push(format!("  Last H2: {last_heading}"));
        lines.push(format!("  Last H2 Content: {last_content}"));
    } else {
        let first = render_recent_activity_snippet(sections.first());
        let last = render_recent_activity_snippet(sections.last());
        lines.push(format!("  First: \"{first}\""));
        lines.push(format!("  Last: \"{last}\""));
    }

    lines.join("\n")
}

fn count_recent_activity_sections(
    file_type: RecentActivityType,
    sections: &[(String, String)],
) -> (&'static str, usize) {
    match file_type {
        RecentActivityType::Sequential | RecentActivityType::Thinking => (
            "Thoughts",
            sections
                .iter()
                .filter(|(heading, _)| heading.starts_with("Thought"))
                .count(),
        ),
        RecentActivityType::Workflow => ("Steps", sections.len()),
        RecentActivityType::Memory => ("Sections", sections.len()),
    }
}

fn section_details(section: Option<&(String, String)>) -> (String, String) {
    match section {
        Some((heading, content)) => (
            heading.trim().to_string(),
            if content.trim().is_empty() {
                String::new()
            } else {
                content.trim().to_string()
            },
        ),
        None => (String::new(), String::new()),
    }
}

// T-2.1: RTM FR-4 / RTM-025 exposes snippet extraction parity for RecentActivity pipeline tests.
pub fn recent_activity_extract_snippet(content: &str, max_length: i32) -> String {
    let max_length = if max_length == -1 {
        std::env::var("MAENIFOLD_SNIPPET_LENGTH")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(1000)
    } else {
        usize::try_from(max_length)
            .ok()
            .filter(|value| *value > 0)
            .unwrap_or(0)
    };
    if max_length == 0 {
        return String::new();
    }

    let lines = content
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    let mut content_lines = lines
        .iter()
        .copied()
        .filter(|line| !line.trim_start().starts_with('*'))
        .collect::<Vec<_>>();
    if content_lines.is_empty() {
        content_lines = lines;
    }

    let snippet = content_lines.join(" ").trim().to_string();
    if snippet.chars().count() <= max_length {
        return snippet;
    }

    if max_length <= 3 {
        return ".".repeat(max_length);
    }
    let prefix = snippet.chars().take(max_length - 3).collect::<String>();
    format!("{prefix}...")
}

fn render_recent_activity_snippet(section: Option<&(String, String)>) -> String {
    let source = match section {
        Some((_, content)) if !content.trim().is_empty() => content,
        Some((heading, _)) => heading,
        None => "",
    };
    recent_activity_extract_snippet(source, -1)
}

fn extract_recent_activity_session_id(file_path: &str) -> String {
    file_path
        .trim_start_matches("memory://")
        .split('/')
        .next_back()
        .unwrap_or(file_path)
        .trim_end_matches(".md")
        .to_string()
}

fn format_recent_activity_timestamp(value: &str) -> String {
    if let Ok(timestamp) = DateTime::parse_from_rfc3339(value) {
        return timestamp
            .with_timezone(&Utc)
            .format("%Y-%m-%d %H:%M")
            .to_string();
    }
    if let Ok(timestamp) = DateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S %Z") {
        return timestamp
            .with_timezone(&Utc)
            .format("%Y-%m-%d %H:%M")
            .to_string();
    }
    if let Some(prefix) = value.get(0..16) {
        return prefix.replace('T', " ");
    }
    value.to_string()
}

pub fn render(format: OutputFormat, value: Value) -> String {
    match format {
        OutputFormat::Json | OutputFormat::Markdown => {
            if let Some(s) = value.as_str() {
                s.to_string()
            } else {
                serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string())
            }
        }
    }
}

fn render_config_markdown(context: &ToolContext) -> String {
    format!(
        "Maenifold Configuration:\n  Memory Path: {}\n  Database: {}\n  Debounce: {}ms\n  Auto Sync: {}\n  Decay Config:\n    Grace Days (Sequential): {}\n    Grace Days (Workflows): {}\n    Grace Days (Default): {}\n    Half-Life Days: {}\n    Function: {}\n  Community Detection:\n    Louvain Gamma: {}\n    Sibling Min Shared Neighbors: {}\n    Sibling Min Overlap: {}\n    Sibling Max Results: {}\n    Watcher Debounce: {}ms\n    NMI Drift Threshold: {}",
        context.config.memory_path.display(),
        context.config.database_path.display(),
        std::env::var("MAENIFOLD_DEBOUNCE_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(150),
        if context.config.enable_incremental_sync() {
            "True"
        } else {
            "False"
        },
        context.config.decay_grace_days_sequential(),
        context.config.decay_grace_days_workflows(),
        context.config.decay_grace_days_default(),
        context.config.decay_half_life_days(),
        context.config.decay_function(),
        std::env::var("MAENIFOLD_LOUVAIN_GAMMA")
            .ok()
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|value| value.is_finite())
            .map(|value| value.to_string())
            .unwrap_or_else(|| "1".to_string()),
        std::env::var("MAENIFOLD_COMMUNITY_MIN_SHARED")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(3),
        std::env::var("MAENIFOLD_COMMUNITY_MIN_OVERLAP")
            .ok()
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(0.4),
        std::env::var("MAENIFOLD_COMMUNITY_MAX_SIBLINGS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(10),
        std::env::var("MAENIFOLD_COMMUNITY_DEBOUNCE_MS")
            .ok()
            .and_then(|value| value.parse::<i64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(2_000),
        std::env::var("MAENIFOLD_COMMUNITY_NMI_DRIFT_THRESHOLD")
            .ok()
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|value| value.is_finite())
            .map(|value| value.to_string())
            .unwrap_or_else(|| "0.8".to_string()),
    )
}
