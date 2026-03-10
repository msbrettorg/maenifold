use crate::config::Config;
use crate::error::MaenifoldError;
use crate::json::{OutputFormat, failure, from_str_safe, success};
use crate::mcp::{self, MaenifoldMcpServer};
use crate::registry::ToolRegistry;
use serde_json::Value;

pub struct CliApp;

impl CliApp {
    pub fn run(args: &[String]) -> i32 {
        if args.iter().any(|arg| arg == "--mcp") {
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

    fn run_inner(args: &[String]) -> std::result::Result<String, (String, i32)> {
        let config = Config::current();
        if let Err(err) = mcp::initialize_runtime(&config) {
            return Err((format!("Error initializing storage: {err}"), 1));
        }

        let Some(tool_index) = args.iter().position(|arg| arg == "--tool") else {
            // T-11.3: RTM FR-13 - invalid direct CLI invocations must fail with the C# parity exit code.
            return Err(usage_error());
        };
        let Some(tool_name) = args.get(tool_index + 1) else {
            // T-11.3: RTM FR-13 - invalid direct CLI invocations must fail with the C# parity exit code.
            return Err(usage_error());
        };
        // T-0.2: RTM NFR-3 - allow side-effect-free direct invocations like
        // `--tool GetConfig --json` by defaulting the payload to `{}` when
        // `--payload` is omitted.
        let payload_json = match args.iter().position(|arg| arg == "--payload") {
            Some(payload_index) => match args.get(payload_index + 1) {
                Some(payload_json) => payload_json.as_str(),
                // T-11.3: RTM FR-13 - invalid direct CLI invocations must fail with the C# parity exit code.
                None => return Err(usage_error()),
            },
            None => "{}",
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
                    // T-11.3: RTM FR-13 - pass through existing JSON envelopes only when
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

fn usage() -> String {
    [
        "maenifold CLI",
        "",
        "Usage:",
        "  MCP Server mode:   maenifold --mcp",
        "  Direct tool mode:  maenifold --tool <name> --payload '<json>' [--json]",
        "",
        "Options:",
        "  --json             Output structured JSON instead of markdown (FR-8.1)",
    ]
    .join("\n")
}
