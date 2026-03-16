use crate::config::Config;
use crate::error::MaenifoldError;
use crate::mcp::{self, MaenifoldMcpServer};
use crate::registry::ToolRegistry;
use crate::sync::watchers;
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, failure, from_str_safe, success};
use serde_json::{Value, json};

/// T-29.2: RTM FR-35 — parsed CLI action for tray-aware dispatch in `main()`.
///
/// [`CliAction::Headless`] covers `--mcp`, `--tool`, `--help`, and error paths —
/// all of which are fully handled by [`CliApp::run`] on the main thread with no
/// native event loop.
///
/// [`CliAction::Http`] signals that `main()` should delegate to
/// [`crate::tray::run_with_tray`] instead.
#[derive(Debug)]
pub enum CliAction {
    /// --mcp, --tool, --help, or invalid args → handled by [`CliApp::run`].
    Headless,
    /// --http mode with the resolved port.
    Http(u16),
}

pub struct CliApp;

impl CliApp {
    /// T-29.2: Classify CLI arguments *before* blocking on any runtime.
    ///
    /// Returns [`CliAction::Http`] only when the flags are unambiguously valid
    /// for that mode. All other cases (including
    /// errors) fall through to [`CliAction::Headless`] so that [`Self::run`]
    /// can print the appropriate error message.
    pub fn classify(args: &[String]) -> CliAction {
        let wants_mcp = args.iter().any(|a| a == "--mcp");
        let wants_http = args.iter().any(|a| a == "--http");

        // Mutual-exclusion violations → let run() handle the error.
        if wants_mcp && wants_http {
            return CliAction::Headless;
        }

        if wants_http {
            return match parse_http_port(args) {
                Ok(port) => CliAction::Http(port),
                Err(_) => CliAction::Headless,
            };
        }

        CliAction::Headless
    }

    pub fn run(args: &[String]) -> i32 {
        // T-16.1: RTM FR-8.1 — explicit --help prints full usage and exits 0 (POSIX convention; C# bug not ported).
        if args.iter().any(|arg| arg == "--help") {
            println!("{}", usage());
            return 0;
        }

        let wants_mcp = args.iter().any(|arg| arg == "--mcp");
        let wants_http = args.iter().any(|arg| arg == "--http");

        // T-17.3: RTM FR-23 (RTM-126, RTM-128) — stdio and HTTP entrypoints
        // are mutually exclusive runtime modes.
        if wants_mcp && wants_http {
            eprintln!("Error: --mcp and --http are mutually exclusive");
            return 1;
        }

        // T-17.3: RTM FR-23 (RTM-126) — reject stray --port usage so the
        // hand-rolled CLI stays unambiguous about HTTP transport options.
        if args.iter().any(|arg| arg == "--port") && !wants_http {
            eprintln!("Error: --port is only valid with --http");
            return 1;
        }

        if wants_http {
            let port = match parse_http_port(args) {
                Ok(port) => port,
                Err(err) => {
                    eprintln!("{err}");
                    return 1;
                }
            };
            return Self::run_http(port);
        }

        if wants_mcp {
            return Self::run_mcp();
        }

        match Self::run_inner(args) {
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

    fn run_mcp() -> i32 {
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

    fn run_http(port: u16) -> i32 {
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

    fn run_inner(args: &[String]) -> std::result::Result<String, (String, i32)> {
        let config = Config::current();
        if let Err(err) = mcp::initialize_runtime(&config) {
            return Err((format!("Error initializing storage: {err}"), 1));
        }

        let Some(tool_index) = args.iter().position(|arg| arg == "--tool") else {
            // T-12.7: RTM NFR-1 - invalid direct CLI invocations must keep the established C#-parity exit code.
            return Err(usage_error());
        };
        let Some(tool_name) = args.get(tool_index + 1) else {
            // T-12.7: RTM NFR-1 - invalid direct CLI invocations must keep the established C#-parity exit code.
            return Err(usage_error());
        };
        // PARITY-CLI-PAYLOAD: RTM NFR-1 — when --tool is present but --payload
        // is missing, print usage and exit 1 (C# parity).
        let payload_json = match args.iter().position(|arg| arg == "--payload") {
            Some(payload_index) => match args.get(payload_index + 1) {
                Some(payload_json) => payload_json.as_str(),
                // T-12.7: RTM NFR-1 - invalid direct CLI invocations must keep the established C#-parity exit code.
                None => return Err(usage_error()),
            },
            None => return Err(usage_error()),
        };

        let output_format = if args.iter().any(|arg| arg == "--json") {
            OutputFormat::Json
        } else {
            OutputFormat::Markdown
        };

        let payload: Value = match from_str_safe(payload_json) {
            Ok(value) => value,
            Err(err) => {
                let output = if output_format == OutputFormat::Json {
                    failure(
                        "INVALID_PAYLOAD",
                        format!("Error invoking {tool_name}: {err}"),
                    )
                } else {
                    format!("Error invoking {tool_name}: {err}")
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
    // T-10.4: RTM FR-12 — mirror C# Program.cs by auto-starting incremental sync
    // before entering MCP stdio mode when auto sync is enabled.
    if config.enable_auto_sync() {
        let context = ToolContext::new(config.clone(), OutputFormat::Markdown);
        let payload = json!({ "debounceMs": 150 });
        watchers::start_watcher(&context, &payload)?;
    }

    Ok(())
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

fn parse_http_port(args: &[String]) -> std::result::Result<u16, String> {
    // T-17.3: RTM FR-23 (RTM-126) — `--http` defaults to port 8080 but accepts
    // an explicit u16 override through the next positional argument after --port.
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

// T-16.1: RTM FR-8.1 — Full help text matching C# parity with dev-from-source note,
// examples, and categorised tool listing.
fn usage() -> String {
    [
        "maenifold CLI",
        "",
        "Usage:",
        "  MCP Server mode:   maenifold --mcp",
        "  HTTP Server mode:  maenifold --http [--port <PORT>]",
        "  Direct tool mode:  maenifold --tool <name> --payload '<json>' [--json]",
        "",
        "  (For development from source: cargo run -p maenifold-cli -- --tool <name> --payload '<json>')",
        "",
        "Options:",
        "  --json             Output structured JSON instead of markdown (FR-8.1)",
        "",
        "Examples:",
        "  maenifold --tool WriteMemory --payload '{\"title\":\"Test\",\"content\":\"Content with [[WikiLink]]\"}'",
        "  maenifold --tool SearchMemories --payload '{\"query\":\"authentication\",\"mode\":\"Hybrid\"}'",
        "  maenifold --tool BuildContext --payload '{\"conceptName\":\"Machine Learning\",\"depth\":2}'",
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
        "  MCP:    ListAssets, ReadMcpResource",
        "  Bench:  RunFullBenchmark",
    ]
    .join("\n")
}
