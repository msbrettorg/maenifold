use std::{cell::RefCell, sync::Arc};

use crate::config::Config;
use crate::error::MaenifoldError;
use crate::mcp::stdio_pool::StdioPool;
use crate::util::json::{OutputFormat, failure};
use serde_json::Value;
use std::fs;
use std::path::Path;
use tokio_util::sync::CancellationToken;

thread_local! {
    static CURRENT_CANCELLATION_TOKEN: RefCell<Option<CancellationToken>> = RefCell::new(None);
}

pub struct ToolContext {
    pub config: Config,
    pub output_format: OutputFormat,
    pub stdio_pool: Option<Arc<StdioPool>>,
    pub cancellation_token: Option<CancellationToken>,
}

impl ToolContext {
    pub fn new(config: Config, output_format: OutputFormat) -> Self {
        Self {
            config,
            output_format,
            stdio_pool: None,
            cancellation_token: current_cancellation_token(),
        }
    }

    pub fn with_stdio_pool(mut self, stdio_pool: Option<Arc<StdioPool>>) -> Self {
        self.stdio_pool = stdio_pool;
        self
    }
}

pub(crate) fn with_cancellation_scope<T>(
    token: CancellationToken,
    action: impl FnOnce() -> T,
) -> T {
    let previous = CURRENT_CANCELLATION_TOKEN.with(|current| current.replace(Some(token)));
    let _guard = CancellationScopeReset { previous };
    action()
}

fn current_cancellation_token() -> Option<CancellationToken> {
    CURRENT_CANCELLATION_TOKEN.with(|current| current.borrow().clone())
}

struct CancellationScopeReset {
    previous: Option<CancellationToken>,
}

impl Drop for CancellationScopeReset {
    fn drop(&mut self) {
        let previous = self.previous.take();
        CURRENT_CANCELLATION_TOKEN.with(|current| {
            current.replace(previous);
        });
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
