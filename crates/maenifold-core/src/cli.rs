mod handlers;
mod parse;
mod tool_dispatch;
mod usage;

use std::path::PathBuf;

use crate::config::Config;
use crate::error::MaenifoldError;
use crate::mcp;
use crate::util::json::{OutputFormat, failure, from_str_safe, success};
use clap::{Parser, Subcommand};
use handlers::{run_http, run_mcp, run_schedule_run, run_tool_command};
use parse::{ParsedCli, parse_cli, render_clap_error, render_error, validate_cli};
use serde_json::Value;
use std::str::FromStr;
use tool_dispatch::invoke_tool_with_timeout;
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

    /// MCP bind address (default: 127.0.0.1)
    #[arg(long, default_value = "127.0.0.1")]
    pub mcp_bind: std::net::IpAddr,

    /// Local API port (default: 8081)
    #[arg(long, default_value_t = 8081)]
    pub local_api_port: u16,

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
    /// Scheduled task supervisor operations
    Schedule {
        #[command(subcommand)]
        command: ScheduleCommand,
    },
}

#[derive(Subcommand, Debug)]
pub enum ScheduleCommand {
    /// Run a scheduled task by id
    Run {
        #[arg(allow_hyphen_values = true)]
        task_id: String,
    },
}

/// T-29.2: RTM FR-35 — parsed CLI action for tray-aware dispatch in `main()`.
///
/// [`CliAction::Headless`] covers `tool`, `--help`, and error paths — all of
/// which are fully handled by [`CliApp::run`] on the main thread with no native
/// event loop.
///
/// [`CliAction::Http`] signals that `main()` should
/// delegate to [`crate::tray::run_with_tray`] instead for `--http` / `--app`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliAction {
    /// tool, --help, or invalid args → handled by [`CliApp::run`].
    Headless,
    /// --mcp mode, optionally carrying a service root when shared by other CLIs.
    Mcp { root: Option<PathBuf> },
    /// --http / --app mode with the resolved port and optional service root.
    Http {
        mcp_bind: String,
        mcp_port: u16,
        local_api_port: u16,
        root: Option<PathBuf>,
    },
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
            let mcp_bind = parsed.cli.mcp_bind.to_string();
            if std::net::IpAddr::from_str(&mcp_bind).is_err() {
                return CliAction::Headless;
            }
            CliAction::Http {
                mcp_bind,
                mcp_port: parsed.cli.port,
                local_api_port: parsed.cli.local_api_port,
                root: None,
            }
        } else if parsed.cli.mcp {
            CliAction::Mcp { root: None }
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
            return run_http(cli.mcp_bind.to_string(), cli.port, cli.local_api_port);
        }

        if cli.app {
            return run_app(cli.mcp_bind.to_string(), cli.port, cli.local_api_port);
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
            Some(CliCommand::Schedule {
                command: ScheduleCommand::Run { task_id },
            }) => run_schedule_run(task_id),
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

        match invoke_tool_with_timeout(tool_name, payload, output_format) {
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

#[allow(dead_code)]
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

#[cfg(feature = "app")]
fn run_app(_mcp_bind: String, port: u16, _local_api_port: u16) -> i32 {
    crate::tray::run_with_tray(crate::tray::ServerMode::Http(port))
}

#[cfg(not(feature = "app"))]
fn run_app(_mcp_bind: String, _port: u16, _local_api_port: u16) -> i32 {
    eprintln!("Error: --app is not supported in this build");
    1
}

#[cfg(test)]
mod tests {
    use super::{CliAction, CliApp};

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn cli_default_mcp_bind_is_127_0_0_1() {
        match CliApp::classify(&args(&["--http", "--port", "8080"])) {
            CliAction::Http { mcp_bind, .. } => assert_eq!(mcp_bind, "127.0.0.1"),
            other => panic!("expected http action, got {other:?}"),
        }
    }

    #[test]
    fn cli_parses_explicit_mcp_bind() {
        match CliApp::classify(&args(&[
            "--http",
            "--port",
            "8080",
            "--mcp-bind",
            "0.0.0.0",
        ])) {
            CliAction::Http { mcp_bind, .. } => assert_eq!(mcp_bind, "0.0.0.0"),
            other => panic!("expected http action, got {other:?}"),
        }
    }

    #[test]
    fn cli_default_local_api_port_is_8081() {
        match CliApp::classify(&args(&["--http", "--port", "8080"])) {
            CliAction::Http { local_api_port, .. } => assert_eq!(local_api_port, 8081),
            other => panic!("expected http action, got {other:?}"),
        }
    }

    #[test]
    fn cli_parses_explicit_local_api_port() {
        match CliApp::classify(&args(&[
            "--http",
            "--port",
            "8080",
            "--local-api-port",
            "9999",
        ])) {
            CliAction::Http { local_api_port, .. } => assert_eq!(local_api_port, 9999),
            other => panic!("expected http action, got {other:?}"),
        }
    }
}
