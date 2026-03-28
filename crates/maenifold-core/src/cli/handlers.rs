use crate::config::Config;
use crate::error::MaenifoldError;
use crate::mcp::{self, MaenifoldMcpServer};
use crate::sync::watchers;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use crate::workflow::runner::{ThinkOptions, run_think, run_workflow};
use serde_json::json;

const WORKFLOW_USAGE: &str = "Usage: maenifold workflow <workflow-id> <agent> <prompt>\n\nRuns a workflow session through the configured ACP agent and prints the conclusion.";
const THINK_USAGE: &str = "Usage: maenifold think <agent> <topic>\n\nRuns a sequential thinking session through the configured ACP agent and prints the conclusion.";

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

    match rt.block_on(MaenifoldMcpServer::serve_stdio()) {
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

    match rt.block_on(MaenifoldMcpServer::serve_http(port)) {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("Error running MCP HTTP server: {err}");
            1
        }
    }
}

pub(super) fn run_script_command(args: &[String]) -> Option<i32> {
    let (command, rest) = args.split_first()?;
    if command.starts_with('-') {
        return None;
    }
    match command.as_str() {
        "workflow" => Some(run_workflow_command(rest)),
        "think" => Some(run_think_command(rest)),
        _ => None,
    }
}

pub(super) fn parse_http_port(args: &[String]) -> std::result::Result<u16, String> {
    // T-17.3: RTM FR-23 (RTM-126) — `--http` defaults to port 8080
    // but accepts an explicit u16 override through the next positional argument
    // after --port.
    let Some(port_index) = args.iter().position(|arg| arg == "--port") else {
        return Ok(8080);
    };

    let Some(port_raw) = args.get(port_index + 1) else {
        return Err("Error: --port requires a value".to_string());
    };

    port_raw
        .parse::<u16>()
        .map_err(|_| format!("Error: invalid port '{}'", port_raw))
}

fn run_workflow_command(args: &[String]) -> i32 {
    if matches!(args, [help] if help == "--help") {
        println!("{WORKFLOW_USAGE}");
        return 0;
    }
    let (workflow_id, agent, prompt) = match args {
        [workflow_id, agent, prompt] => (workflow_id.clone(), agent.clone(), prompt.clone()),
        _ => {
            eprintln!("{WORKFLOW_USAGE}");
            return 1;
        }
    };
    run_async_command(move |context| async move {
        run_workflow(&context, &workflow_id, &agent, &prompt)
            .await
            .map(|result| result.conclusion.unwrap_or_default())
    })
}

fn run_think_command(args: &[String]) -> i32 {
    if matches!(args, [help] if help == "--help") {
        println!("{THINK_USAGE}");
        return 0;
    }
    let (agent, topic) = match args {
        [agent, topic] => (agent.clone(), topic.clone()),
        _ => {
            eprintln!("{THINK_USAGE}");
            return 1;
        }
    };
    run_async_command(move |context| async move {
        run_think(&context, &agent, &topic, ThinkOptions::default())
            .await
            .map(|result| result.conclusion.unwrap_or_default())
    })
}

fn run_async_command<F, Fut>(f: F) -> i32
where
    F: FnOnce(ToolContext) -> Fut,
    Fut: std::future::Future<Output = crate::error::Result<String>>,
{
    let config = Config::current();
    if let Err(err) = mcp::initialize_runtime(&config) {
        eprintln!("Error initializing storage: {err}");
        return 1;
    }
    let context = ToolContext::new(config, OutputFormat::Markdown);
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(err) => {
            eprintln!("Error creating async runtime: {err}");
            return 1;
        }
    };
    match rt.block_on(f(context)) {
        Ok(output) => {
            println!("{output}");
            0
        }
        Err(err) => {
            eprintln!("{err}");
            1
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
