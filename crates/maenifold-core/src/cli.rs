mod handlers;
mod parse;
mod usage;

use crate::config::Config;
use crate::error::MaenifoldError;
use crate::mcp;
use crate::registry::ToolRegistry;
use crate::util::json::{OutputFormat, failure, from_str_safe, success};
use clap::{Parser, Subcommand};
use handlers::{run_http, run_mcp, run_tool_command};
use parse::{ParsedCli, parse_cli, render_clap_error, render_error, validate_cli};
use serde_json::Value;
use usage::usage;

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
    /// Invoke a maenifold tool directly
    Tool {
        name: String,
        payload: Option<String>,
        #[arg(long)]
        json: bool,
    },
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

        if blocks_headless_pipeline_run(tool_name, &payload) {
            let message = "Error invoking Pipeline: Pipeline run=true requires HTTP/server mode because the headless CLI process exits immediately. Use the HTTP server path instead.";
            let output = if output_format == OutputFormat::Json {
                failure("PIPELINE_RUN_REQUIRES_HTTP_MODE", message)
            } else {
                message.to_string()
            };
            return Err((output, 1));
        }

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

fn blocks_headless_pipeline_run(tool_name: &str, payload: &Value) -> bool {
    if !tool_name.eq_ignore_ascii_case("Pipeline") {
        return false;
    }

    payload
        .as_object()
        .filter(|object| {
            object.get("run").and_then(Value::as_bool) == Some(true)
                || object.get("resume").and_then(Value::as_bool) == Some(true)
        })
        .and_then(|object| object.get("sessionId"))
        .and_then(Value::as_str)
        .map(str::trim)
        .is_some_and(|session_id| !session_id.is_empty())
}

#[cfg(test)]
mod tests {
    use super::blocks_headless_pipeline_run;
    use serde_json::json;

    #[test]
    fn blocks_headless_pipeline_run_allows_run_requests() {
        assert!(!blocks_headless_pipeline_run(
            "Pipeline",
            &json!({
                "sessionId": "pipeline-1700000000000-cli",
                "run": true
            }),
        ));
    }

    #[test]
    fn blocks_headless_pipeline_run_still_blocks_resume_requests() {
        assert!(blocks_headless_pipeline_run(
            "Pipeline",
            &json!({
                "sessionId": "pipeline-1700000000000-cli",
                "resume": true
            }),
        ));
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
