mod formatting;
mod handler;
mod session;
mod session_init;
mod validation;

pub use handler::sequential_thinking;
pub(crate) use session::sequential_session_uri;

use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, failure, success};

// T-12.0: RTM NFR-1 - preserve the public SequentialThinking entrypoint while splitting helpers.
pub(crate) fn render_failure(
    context: &ToolContext,
    code: &str,
    message: impl AsRef<str>,
) -> String {
    let message = message.as_ref();
    match context.output_format {
        OutputFormat::Json => failure(code, message),
        OutputFormat::Markdown => format!("ERROR: {message}"),
    }
}

pub(crate) fn render_text_success(context: &ToolContext, markdown: impl Into<String>) -> String {
    let markdown = markdown.into();
    match context.output_format {
        OutputFormat::Json => success(markdown.clone()),
        OutputFormat::Markdown => markdown,
    }
}
