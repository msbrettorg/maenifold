use std::sync::Arc;

use crate::config::Config;
use crate::error::MaenifoldError;
use crate::mcp::stdio_pool::StdioPool;
use crate::util::json::{OutputFormat, failure};
use serde_json::Value;
use std::fs;
use std::path::Path;

pub struct ToolContext {
    pub config: Config,
    pub output_format: OutputFormat,
    pub stdio_pool: Option<Arc<StdioPool>>,
}

impl ToolContext {
    pub fn new(config: Config, output_format: OutputFormat) -> Self {
        Self {
            config,
            output_format,
            stdio_pool: None,
        }
    }

    pub fn with_stdio_pool(mut self, stdio_pool: Option<Arc<StdioPool>>) -> Self {
        self.stdio_pool = stdio_pool;
        self
    }
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

pub(crate) fn load_help_doc(assets_path: &Path, tool_name: &str) -> Option<String> {
    fs::read_to_string(
        assets_path
            .join("usage")
            .join("tools")
            .join(format!("{}.md", tool_name.to_ascii_lowercase())),
    )
    .ok()
}

pub(crate) fn required_string<'a>(
    payload: &'a Value,
    key: &'static str,
) -> Result<&'a str, MaenifoldError> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty(key))
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
