use crate::config::Config;
use crate::error::MaenifoldError;
use crate::mcp::{self, MaenifoldMcpServer};
use crate::sync::watchers;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use serde_json::json;

pub(super) fn run_mcp() -> i32 {
    let config = Config::current();
    if let Err(err) = mcp::initialize_runtime(&config) {
        eprintln!("Error initializing storage: {err}");
        return 1;
    }
    if let Err(err) = start_mcp_watchers(&config) {
        eprintln!("Error starting MCP watchers: {err}");
        return 1;
    }

    // T-5.1: RTM FR-22.1 — rmcp requires a tokio async runtime for stdio transport.
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(err) => {
            eprintln!("Error creating async runtime: {err}");
            return 1;
        }
    };

    let result = rt.block_on(MaenifoldMcpServer::serve_stdio());
    rt.shutdown_timeout(std::time::Duration::from_secs(5));
    match result {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("Error running MCP stdio server: {err}");
            1
        }
    }
}

pub(super) fn run_http(port: u16) -> i32 {
    let config = Config::current();
    if let Err(err) = mcp::initialize_runtime(&config) {
        eprintln!("Error initializing storage: {err}");
        return 1;
    }
    if let Err(err) = start_mcp_watchers(&config) {
        eprintln!("Error starting MCP watchers: {err}");
        return 1;
    }

    // T-17.3: RTM FR-23 (RTM-126, RTM-128) — HTTP transport uses the same
    // async runtime/bootstrap path as stdio before handing off to serve_http.
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(err) => {
            eprintln!("Error creating async runtime: {err}");
            return 1;
        }
    };

    let result = rt.block_on(MaenifoldMcpServer::serve_http(port));
    rt.shutdown_timeout(std::time::Duration::from_secs(5));
    match result {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("Error running MCP HTTP server: {err}");
            1
        }
    }
}

pub(super) fn run_tool_command(
    name: String,
    payload: Option<String>,
    json: bool,
    warn_deprecated: bool,
) -> i32 {
    let output_format = if json {
        OutputFormat::Json
    } else {
        OutputFormat::Markdown
    };
    match super::CliApp::run_tool_inner(&name, payload.as_deref(), output_format, warn_deprecated) {
        Ok(output) => {
            println!("{output}");
            0
        }
        Err((output, code)) => {
            println!("{output}");
            code
        }
    }
}

pub(crate) fn start_mcp_watchers(config: &Config) -> Result<(), MaenifoldError> {
    // T-10.4: RTM FR-12 — mirror C# Program.cs by auto-starting incremental sync
    // before entering MCP stdio mode when auto sync is enabled.
    if config.enable_auto_sync() {
        let context = ToolContext::new(config.clone(), OutputFormat::Markdown);
        let payload = json!({ "debounceMs": 150 });
        watchers::start_watcher(&context, &payload)?;
    }

    Ok(())
}
