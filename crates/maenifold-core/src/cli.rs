mod handlers;

use crate::config::Config;
use crate::error::MaenifoldError;
use crate::mcp;
use crate::registry::ToolRegistry;
use crate::util::json::{OutputFormat, failure, from_str_safe, success};
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand, parser::ValueSource};
use handlers::{
    run_http, run_mcp, run_prompt_command, run_think_command, run_tool_command,
    run_workflow_command,
};
use serde_json::Value;

#[derive(Parser, Debug)]
#[command(
    name = "maenifold",
    version,
    about = "Maenifold knowledge graph and AI orchestration",
    disable_help_subcommand = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<CliCommand>,

    /// Run as MCP server on stdio
    #[arg(long)]
    pub mcp: bool,

    /// Run HTTP server
    #[arg(long)]
    pub http: bool,

    /// HTTP port (default: 8080)
    #[arg(long, default_value_t = 8080)]
    pub port: u16,

    /// Run with native app window
    #[arg(long)]
    pub app: bool,
}

#[derive(Subcommand, Debug)]
pub enum CliCommand {
    /// Send a prompt to an ACP agent
    Prompt {
        agent: String,
        message: String,
        #[arg(long)]
        model: Option<String>,
    },
    /// Run a workflow through an ACP agent
    Workflow {
        workflow_id: String,
        agent: String,
        prompt: String,
        #[arg(long)]
        model: Option<String>,
    },
    /// Run a sequential thinking session
    Think {
        agent: String,
        topic: String,
        #[arg(long)]
        model: Option<String>,
    },
    /// Invoke a maenifold tool directly
    Tool {
        name: String,
        payload: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

struct ParsedCli {
    cli: Cli,
    port_was_explicit: bool,
    warn_deprecated_tool: bool,
}

/// T-29.2: RTM FR-35 — parsed CLI action for tray-aware dispatch in `main()`.
///
/// [`CliAction::Headless`] covers `--mcp`, `tool`, `--help`, and error paths —
/// all of which are fully handled by [`CliApp::run`] on the main thread with no
/// native event loop.
///
/// [`CliAction::Http`] signals that `main()` should
/// delegate to [`crate::tray::run_with_tray`] instead for `--http` / `--app`.
#[derive(Debug)]
pub enum CliAction {
    /// --mcp, tool, --help, or invalid args → handled by [`CliApp::run`].
    Headless,
    /// --http / --app mode with the resolved port.
    Http(u16),
}
pub struct CliApp;
impl CliApp {
    /// T-29.2: Classify CLI arguments *before* blocking on any runtime.
    ///
    /// Returns [`CliAction::Http`] only when the flags
    /// are unambiguously valid for that mode.  All other cases (including
    /// errors) fall through to [`CliAction::Headless`] so that [`Self::run`]
    /// can print the appropriate error message.
    pub fn classify(args: &[String]) -> CliAction {
        let Ok(parsed) = parse_cli(args) else {
            return CliAction::Headless;
        };
        if validate_cli(&parsed).is_err() {
            return CliAction::Headless;
        }
        if parsed.cli.http || parsed.cli.app {
            CliAction::Http(parsed.cli.port)
        } else {
            CliAction::Headless
        }
    }

    pub fn run(args: &[String]) -> i32 {
        let parsed = match parse_cli(args) {
            Ok(parsed) => parsed,
            Err(err) => return render_clap_error(err),
        };
        if let Err(err) = validate_cli(&parsed) {
            eprintln!("{err}");
            return 1;
        }

        let ParsedCli {
            cli,
            warn_deprecated_tool,
            ..
        } = parsed;

        if cli.http {
            return run_http(cli.port);
        }

        if cli.app {
            return run_app(cli.port);
        }

        if cli.mcp {
            return run_mcp();
        }

        match cli.command {
            Some(CliCommand::Prompt {
                agent,
                message,
                model,
            }) => run_prompt_command(agent, message, model),
            Some(CliCommand::Workflow {
                workflow_id,
                agent,
                prompt,
                model,
            }) => run_workflow_command(workflow_id, agent, prompt, model),
            Some(CliCommand::Think {
                agent,
                topic,
                model,
            }) => run_think_command(agent, topic, model),
            Some(CliCommand::Tool {
                name,
                payload,
                json,
            }) => run_tool_command(name, payload, json, warn_deprecated_tool),
            None => {
                println!("{}", usage());
                1
            }
        }
    }

    fn run_tool_inner(
        tool_name: &str,
        payload_json: Option<&str>,
        output_format: OutputFormat,
        warn_deprecated: bool,
    ) -> std::result::Result<String, (String, i32)> {
        let config = Config::current();
        if let Err(err) = mcp::initialize_runtime(&config) {
            return Err((format!("Error initializing storage: {err}"), 1));
        }

        if warn_deprecated {
            eprintln!(
                "warning: --tool is deprecated, use: maenifold tool <name> '<payload>' [--json]"
            );
        }

        // PARITY-CLI-PAYLOAD: RTM NFR-1 — when tool payload is omitted,
        // print usage and exit 1 except for MemoryStatus which accepts `{}`.
        let payload_json = match payload_json {
            Some(payload_json) => payload_json,
            None if tool_name == "MemoryStatus" => "{}",
            None => return Err(usage_error()),
        };

        let payload: Value = match from_str_safe(payload_json) {
            Ok(value) => value,
            Err(err) => {
                let output = if output_format == OutputFormat::Json {
                    failure(
                        "INVALID_PAYLOAD",
                        format!("Error invoking {}: {err}", tool_name),
                    )
                } else {
                    format!("Error invoking {}: {err}", tool_name)
                };
                return Err((output, 1));
            }
        };

        match ToolRegistry::global().try_invoke(tool_name, &payload, output_format) {
            Ok(output) => {
                if output_format == OutputFormat::Json {
                    // T-12.7: RTM NFR-1 - pass through existing JSON envelopes only when
                    // the tool already returned the full {success,data,error} contract;
                    // otherwise wrap the tool output in the C#-compatible success envelope.
                    let (output, exit_code) = normalize_json_output(output);
                    if exit_code == 0 {
                        Ok(output)
                    } else {
                        Err((output, exit_code))
                    }
                } else {
                    Ok(output)
                }
            }
            Err(err) => {
                let output = render_error(output_format, tool_name, err);
                Err((output, 1))
            }
        }
    }
}

pub(crate) fn start_mcp_watchers(config: &Config) -> Result<(), MaenifoldError> {
    handlers::start_mcp_watchers(config)
}

fn render_error(output_format: OutputFormat, tool_name: &str, err: MaenifoldError) -> String {
    match output_format {
        OutputFormat::Json => match err {
            MaenifoldError::UnknownTool(_) => {
                failure("UNKNOWN_TOOL", format!("Unknown tool: {tool_name}"))
            }
            MaenifoldError::InvalidArgument(msg) => failure(
                "INVALID_ARGUMENT",
                format!("Error invoking {tool_name}: {msg}"),
            ),
            // T-0.3h: RTM FR-1 / FR-2 / FR-4 / FR-5 / FR-6 preserves the C# required-property wording in JSON envelopes.
            MaenifoldError::MissingProperty(msg) => failure(
                "INVALID_ARGUMENT",
                format!(
                    "Error invoking {tool_name}: Required property '{msg}' is missing from payload."
                ),
            ),
            MaenifoldError::InvalidPayload(msg) => failure(
                "INVALID_PAYLOAD",
                format!("Error invoking {tool_name}: {msg}"),
            ),
            MaenifoldError::Io(ref io_err)
                if io_err.raw_os_error() == Some(63)
                    || io_err.to_string().contains("name too long") =>
            {
                failure(
                    "PATH_TOO_LONG",
                    format!("Error invoking {tool_name}: File name too long"),
                )
            }
            other => failure(
                "INTERNAL_ERROR",
                format!("Error invoking {tool_name}: {other}"),
            ),
        },
        OutputFormat::Markdown => match err {
            MaenifoldError::UnknownTool(_) => format!("Unknown tool: {tool_name}"),
            other => format!("Error invoking {tool_name}: {other}"),
        },
    }
}

fn usage_error() -> (String, i32) {
    (usage(), 1)
}

fn normalize_json_output(output: String) -> (String, i32) {
    match serde_json::from_str::<Value>(&output) {
        Ok(Value::Object(object))
            if object.contains_key("success")
                && object.contains_key("data")
                && object.contains_key("error") =>
        {
            let exit_code = if object.get("success").and_then(Value::as_bool) == Some(false) {
                1
            } else {
                0
            };
            (output, exit_code)
        }
        Ok(value) => (success(value), 0),
        Err(_) => (success(output), 0),
    }
}

fn parse_cli(args: &[String]) -> Result<ParsedCli, clap::Error> {
    let normalized = normalize_args(args);
    let argv = std::iter::once("maenifold".to_string()).chain(normalized.args.iter().cloned());
    let matches = Cli::command().try_get_matches_from(argv)?;
    let port_was_explicit = matches.value_source("port") == Some(ValueSource::CommandLine);
    let cli = Cli::from_arg_matches(&matches)?;
    Ok(ParsedCli {
        cli,
        port_was_explicit,
        warn_deprecated_tool: normalized.warn_deprecated_tool,
    })
}

fn validate_cli(parsed: &ParsedCli) -> Result<(), String> {
    let runtime_mode_count =
        usize::from(parsed.cli.mcp) + usize::from(parsed.cli.http) + usize::from(parsed.cli.app);

    if runtime_mode_count > 1 {
        return Err("Error: --mcp, --http, and --app are mutually exclusive".to_string());
    }

    if parsed.cli.command.is_some() && runtime_mode_count > 0 {
        return Err("Error: runtime flags cannot be combined with a subcommand".to_string());
    }

    if parsed.port_was_explicit && !(parsed.cli.http || parsed.cli.app) {
        return Err("Error: --port is only valid with --http or --app".to_string());
    }

    Ok(())
}

fn render_clap_error(err: clap::Error) -> i32 {
    let code = if matches!(
        err.kind(),
        clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
    ) {
        0
    } else {
        2
    };

    if err.use_stderr() {
        eprint!("{err}");
    } else {
        print!("{err}");
    }

    code
}

struct NormalizedArgs {
    args: Vec<String>,
    warn_deprecated_tool: bool,
}

fn normalize_args(args: &[String]) -> NormalizedArgs {
    let Some(tool_index) = args.iter().position(|arg| arg == "--tool") else {
        return NormalizedArgs {
            args: args.to_vec(),
            warn_deprecated_tool: false,
        };
    };

    let mut normalized = vec!["tool".to_string()];
    let mut consumed = vec![tool_index];

    if let Some(tool_name) = args.get(tool_index + 1) {
        normalized.push(tool_name.clone());
        consumed.push(tool_index + 1);
    }

    if let Some(payload_index) = args.iter().position(|arg| arg == "--payload") {
        consumed.push(payload_index);
        if let Some(payload) = args.get(payload_index + 1) {
            normalized.push(payload.clone());
            consumed.push(payload_index + 1);
        }
    }

    for (index, arg) in args.iter().enumerate() {
        if !consumed.contains(&index) {
            normalized.push(arg.clone());
        }
    }

    NormalizedArgs {
        args: normalized,
        warn_deprecated_tool: true,
    }
}

#[cfg(feature = "app")]
fn run_app(port: u16) -> i32 {
    crate::tray::run_with_tray(crate::tray::ServerMode::Http(port))
}

#[cfg(not(feature = "app"))]
fn run_app(_port: u16) -> i32 {
    eprintln!("Error: --app is not supported in this build");
    1
}

// T-16.1: RTM FR-8.1 — Full help text matching C# parity with dev-from-source note,
// examples, and categorised tool listing.
fn usage() -> String {
    [
        "maenifold CLI",
        "",
        "Usage:",
        "  MCP Server mode:   maenifold --mcp",
        "  HTTP Server mode:  maenifold --http [--port <PORT>]",
        "  App mode:          maenifold --app [--port <PORT>]",
        "  Prompt mode:       maenifold prompt <agent> <message> [--model <model>]",
        "  Think mode:        maenifold think <agent> <topic>",
        "  Workflow mode:     maenifold workflow <workflow-id> <agent> <prompt>",
        "  Tool mode:         maenifold tool <name> '<json-payload>' [--json]",
        "",
        "  (For development from source: cargo run -p maenifold-cli -- tool <name> '<json>')",
        "",
        "Options:",
        "  --json             Output structured JSON instead of markdown (FR-8.1)",
        "",
        "Examples:",
        "  maenifold tool WriteMemory '{\"title\":\"Test\",\"content\":\"Content with [[WikiLink]]\"}'",
        "  maenifold tool SearchMemories '{\"query\":\"authentication\",\"mode\":\"Hybrid\"}'",
        "  maenifold tool BuildContext '{\"conceptName\":\"Machine Learning\",\"depth\":2}'",
        "",
        "Available tools:",
        "  Memory: WriteMemory, ReadMemory, EditMemory, DeleteMemory, MoveMemory",
        "          SearchMemories, ExtractConceptsFromFile, RecentActivity",
        "  Vector: FindSimilarConcepts",
        "  Graph:  Sync, BuildContext, Visualize",
        "  Think:  SequentialThinking",
        // T-16.1: RTM FR-8.1 — C# help lists only Workflow under Flow.
        "  Flow:   Workflow",
        "  System: MemoryStatus, ListMemories, GetConfig, GetHelp",
        "  Sync:   StartWatcher, StopWatcher",
        "  Repair: RepairConcepts, AnalyzeConceptCorruption",
        // T-16.1: RTM FR-8.1 — C# groups ListAssets + ReadMcpResource under MCP, not Adopt.
        "  MCP:    ListAssets, ReadMcpResource, ListAgents",
        "  Bench:  RunFullBenchmark",
    ]
    .join("\n")
}
