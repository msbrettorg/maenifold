use crate::acp_config::AgentsConfig;
use crate::acp_native::{AgentPool, SessionOpts};
use crate::config::Config;
use crate::error::MaenifoldError;
use crate::mcp::{self, MaenifoldMcpServer};
use crate::sync::watchers;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use crate::workflow::runner::{ThinkOptions, run_think, run_workflow};
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

pub(super) fn run_workflow_command(workflow_id: String, agent: String, prompt: String) -> i32 {
    run_async_command(move |context| async move {
        run_workflow(&context, &workflow_id, &agent, &prompt, Some("gpt-5.4".to_string()))
            .await
            .map(|result| result.conclusion.unwrap_or_default())
    })
}

pub(super) fn run_prompt_command(agent: String, message: String, model: Option<String>) -> i32 {
    let model = model.or_else(|| Some("gpt-5.4".to_string()));
    run_async_command(move |context| async move {
        let pool = AgentPool::new();
        let config_entry = AgentsConfig::load(&context.config.maenifold_root)
            .agents
            .get(&agent)
            .cloned()
            .ok_or_else(|| {
                MaenifoldError::Message(format!("agent '{}' not configured in agents.json", agent))
            })?;
        let command = config_entry.command;
        let env = config_entry.env;
        let handle = pool
            .get_or_create(&agent, &command, &context.config.maenifold_root, env)
            .await;
        let remote_id = handle
            .new_session(SessionOpts {
                cwd: context.config.maenifold_root.clone(),
                model: model.clone(),
                mode: None,
                thought_level: None,
                config_options: None,
                allowed_tools: None,
                max_turns: None,
                timeout_seconds: None,
                mcp_servers: vec![],
            })
            .await
            .map_err(|error| MaenifoldError::Message(error.to_string()))?
            .session_id;
        let response = handle.prompt(&remote_id, &message).await;
        handle.close_session(&remote_id).await;
        let response = response.map_err(|error| MaenifoldError::Message(error.to_string()))?;
        Ok(response.text)
    })
}

pub(super) fn run_think_command(agent: String, topic: String) -> i32 {
    run_async_command(move |context| async move {
        run_think(
            &context,
            &agent,
            &topic,
            ThinkOptions { model: Some("gpt-5.4".to_string()), ..ThinkOptions::default() },
        )
        .await
        .map(|result| result.conclusion.unwrap_or_default())
    })
}

pub(super) fn run_tool_command(
    name: String,
    payload: Option<String>,
    json: bool,
    warn_deprecated: bool,
) -> i32 {
    let output_format = if json { OutputFormat::Json } else { OutputFormat::Markdown };
    match super::CliApp::run_tool_inner(&name, payload.as_deref(), output_format, warn_deprecated)
    {
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
    let result = rt.block_on(f(context));
    rt.shutdown_timeout(std::time::Duration::from_secs(5));
    match result {
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
