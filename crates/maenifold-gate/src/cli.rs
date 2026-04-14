use std::path::PathBuf;

use clap::{Args, CommandFactory, FromArgMatches, Parser, Subcommand, parser::ValueSource};
pub use maenifold_core::CliAction;

use crate::append_command::AppendCommand;
use crate::error::GateError;
use crate::fail_command::FailCommand;
use crate::history_command::HistoryCommand;
use crate::pass_command::PassCommand;
use crate::status_command::StatusCommand;
use crate::status_filter::StatusFilter;
use crate::sync_command::SyncCommand;
use crate::ui_window::GateUiWindow;

#[derive(Debug, Clone)]
pub struct GateContext {
    pub root: PathBuf,
    pub requested_pipeline_name: Option<String>,
    pub json_output: bool,
}

#[derive(Debug, Clone)]
pub struct StatusArgs {
    pub context: GateContext,
    pub filter: StatusFilter,
}

#[derive(Debug, Clone)]
pub struct AppendArgs {
    pub context: GateContext,
    pub task_id: String,
    pub agent: String,
    pub notes: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HistoryArgs {
    pub context: GateContext,
    pub task_id: String,
}

#[derive(Debug, Clone)]
pub struct TaskBodyArgs {
    pub context: GateContext,
    pub task_id: String,
}

#[derive(Debug, Clone)]
pub struct OutcomeArgs {
    pub context: GateContext,
    pub task_id: String,
    pub agent: String,
    pub notes: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SyncArgs {
    pub context: GateContext,
    pub rebuild: bool,
}

#[derive(Debug, Clone)]
pub enum CommandRequest {
    Status(StatusArgs),
    Append(AppendArgs),
    History(HistoryArgs),
    Pass(OutcomeArgs),
    Fail(OutcomeArgs),
    Sync(SyncArgs),
}

pub struct GateCliApp;

#[derive(Debug)]
struct ParsedCli {
    cli: GateCli,
    port_was_explicit: bool,
}

impl GateCliApp {
    pub fn classify(args: &[String]) -> CliAction {
        let Ok(parsed) = parse_cli(args) else {
            return CliAction::Headless;
        };
        if validate_cli(&parsed).is_err() {
            return CliAction::Headless;
        }
        if parsed.cli.http {
            CliAction::Http {
                port: parsed.cli.port,
                root: parsed.cli.service_root.clone(),
            }
        } else if parsed.cli.mcp {
            CliAction::Mcp {
                root: parsed.cli.service_root.clone(),
            }
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

        match parsed.cli.command {
            Some(GateSubcommand::Ui(args)) => GateUiWindow::run(args.shared.root),
            Some(command) => match command.into_request() {
                Ok(request) => run_command(request),
                Err(err) => {
                    eprintln!("{err}");
                    1
                }
            },
            None => {
                println!("{}", usage());
                1
            }
        }
    }
}

fn run_command(request: CommandRequest) -> i32 {
    let result = match request {
        CommandRequest::Status(args) => StatusCommand::execute(args),
        CommandRequest::Append(args) => AppendCommand::execute(args),
        CommandRequest::History(args) => HistoryCommand::execute(args),
        CommandRequest::Pass(args) => PassCommand::execute(args),
        CommandRequest::Fail(args) => FailCommand::execute(args),
        CommandRequest::Sync(args) => SyncCommand::execute(args),
    };

    match result {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

#[derive(Debug, Parser)]
#[command(name = "gate")]
#[command(about = "File-authoritative gated delivery CLI")]
struct GateCli {
    #[command(subcommand)]
    command: Option<GateSubcommand>,

    #[arg(long)]
    mcp: bool,

    #[arg(long)]
    http: bool,

    #[arg(long, default_value_t = 8081)]
    port: u16,

    #[arg(long = "root")]
    service_root: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
enum GateSubcommand {
    Status(StatusCliArgs),
    Append(AppendCliArgs),
    History(HistoryCliArgs),
    Pass(OutcomeCliArgs),
    Fail(OutcomeCliArgs),
    Sync(SyncCliArgs),
    Ui(UiCliArgs),
}

impl GateSubcommand {
    fn into_request(self) -> Result<CommandRequest, GateError> {
        Ok(match self {
            Self::Status(args) => CommandRequest::Status(StatusArgs {
                context: args.shared.into_context(args.json),
                filter: StatusFilter {
                    state: args.state,
                    ready_only: args.ready,
                    include_done: args.all,
                },
            }),
            Self::Append(args) => CommandRequest::Append(AppendArgs {
                context: args.shared.into_context(false),
                task_id: args.task_id,
                agent: args.agent,
                notes: args.notes,
                model: args.model,
            }),
            Self::History(args) => CommandRequest::History(HistoryArgs {
                context: args.shared.into_context(false),
                task_id: args.task_id,
            }),
            Self::Pass(args) => CommandRequest::Pass(args.into_outcome_args()),
            Self::Fail(args) => CommandRequest::Fail(args.into_outcome_args()),
            Self::Sync(args) => CommandRequest::Sync(SyncArgs {
                context: args.shared.into_context(false),
                rebuild: args.rebuild,
            }),
            Self::Ui(_) => {
                return Err(GateError::Cli(
                    "ui subcommand must be handled directly".to_string(),
                ));
            }
        })
    }
}

#[derive(Debug, Clone, Args)]
struct SharedCliArgs {
    #[arg(long, default_value = ".")]
    root: PathBuf,
    #[arg(long)]
    pipeline: Option<String>,
}

impl SharedCliArgs {
    fn into_context(self, json_output: bool) -> GateContext {
        GateContext {
            root: self.root,
            requested_pipeline_name: self.pipeline,
            json_output,
        }
    }
}

#[derive(Debug, Args)]
struct StatusCliArgs {
    #[command(flatten)]
    shared: SharedCliArgs,
    #[arg(long)]
    json: bool,
    #[arg(long)]
    all: bool,
    #[arg(long)]
    state: Option<String>,
    #[arg(long)]
    ready: bool,
}

#[derive(Debug, Args)]
struct AppendCliArgs {
    task_id: String,
    #[command(flatten)]
    shared: SharedCliArgs,
    #[arg(long)]
    agent: String,
    #[arg(long)]
    notes: String,
    #[arg(long)]
    model: Option<String>,
}

#[derive(Debug, Args)]
struct HistoryCliArgs {
    task_id: String,
    #[command(flatten)]
    shared: SharedCliArgs,
}

#[derive(Debug, Args)]
struct OutcomeCliArgs {
    task_id: String,
    #[command(flatten)]
    shared: SharedCliArgs,
    #[arg(long)]
    agent: String,
    #[arg(long)]
    notes: String,
    #[arg(long)]
    model: Option<String>,
}

impl OutcomeCliArgs {
    fn into_outcome_args(self) -> OutcomeArgs {
        OutcomeArgs {
            context: self.shared.into_context(false),
            task_id: self.task_id,
            agent: self.agent,
            notes: self.notes,
            model: self.model,
        }
    }
}

#[derive(Debug, Args)]
struct SyncCliArgs {
    #[command(flatten)]
    shared: SharedCliArgs,
    #[arg(long)]
    rebuild: bool,
}

#[derive(Debug, Args)]
struct UiCliArgs {
    #[command(flatten)]
    shared: SharedCliArgs,
}

fn parse_cli(args: &[String]) -> Result<ParsedCli, clap::Error> {
    let argv = std::iter::once("gate".to_string()).chain(args.iter().cloned());
    let matches = GateCli::command().try_get_matches_from(argv)?;
    let port_was_explicit = matches.value_source("port") == Some(ValueSource::CommandLine);
    let cli = GateCli::from_arg_matches(&matches)?;
    Ok(ParsedCli {
        cli,
        port_was_explicit,
    })
}

fn validate_cli(parsed: &ParsedCli) -> Result<(), GateError> {
    if parsed.cli.mcp && parsed.cli.http {
        return Err(GateError::Cli(
            "--mcp and --http are mutually exclusive".to_string(),
        ));
    }
    if parsed.port_was_explicit && !parsed.cli.http {
        return Err(GateError::Cli(
            "--port may only be used with --http".to_string(),
        ));
    }
    if parsed.cli.service_root.is_some() && !(parsed.cli.mcp || parsed.cli.http) {
        return Err(GateError::Cli(
            "top-level --root may only be used with --mcp or --http".to_string(),
        ));
    }
    if (parsed.cli.mcp || parsed.cli.http) && parsed.cli.command.is_some() {
        return Err(GateError::Cli(
            "service modes do not accept gate subcommands".to_string(),
        ));
    }
    Ok(())
}

fn usage() -> String {
    let mut command = GateCli::command();
    let mut buffer = Vec::new();
    command
        .write_long_help(&mut buffer)
        .expect("gate clap help should render");
    String::from_utf8(buffer).expect("clap help should be utf-8")
}

fn render_clap_error(error: clap::Error) -> i32 {
    let code = if error.use_stderr() { 1 } else { 0 };
    if error.use_stderr() {
        eprintln!("{error}");
    } else {
        println!("{error}");
    }
    code
}

#[cfg(test)]
mod tests {
    use super::{CliAction, CommandRequest, GateCli, GateCliApp, parse_cli, validate_cli};
    use clap::Parser;

    #[test]
    fn classify_mcp_mode_as_tray_enabled() {
        let args = vec!["--mcp".to_string()];
        assert_eq!(GateCliApp::classify(&args), CliAction::Mcp { root: None });
    }

    #[test]
    fn classify_http_mode_as_tray_enabled() {
        let args = vec![
            "--http".to_string(),
            "--port".to_string(),
            "8181".to_string(),
        ];
        assert_eq!(
            GateCliApp::classify(&args),
            CliAction::Http {
                port: 8181,
                root: None,
            }
        );
    }

    #[test]
    fn classify_service_mode_with_root() {
        let args = vec![
            "--http".to_string(),
            "--root".to_string(),
            "/workspace/project".to_string(),
        ];
        assert_eq!(
            GateCliApp::classify(&args),
            CliAction::Http {
                port: 8081,
                root: Some("/workspace/project".into()),
            }
        );
    }

    #[test]
    fn reject_port_without_http() {
        let args = vec!["--port".to_string(), "8181".to_string()];
        let parsed = parse_cli(&args).expect("CLI should parse");
        let error = validate_cli(&parsed).expect_err("port without --http must fail");
        assert!(
            error
                .to_string()
                .contains("--port may only be used with --http")
        );
    }

    #[test]
    fn reject_top_level_root_without_service_mode() {
        let args = vec!["--root".to_string(), "/workspace/project".to_string()];
        let parsed = parse_cli(&args).expect("CLI should parse");
        let error =
            validate_cli(&parsed).expect_err("top-level root without service mode must fail");
        assert!(
            error
                .to_string()
                .contains("top-level --root may only be used with --mcp or --http")
        );
    }

    #[test]
    fn subcommand_root_remains_valid_without_service_mode() {
        let args = vec![
            "status".to_string(),
            "--root".to_string(),
            "/workspace/project".to_string(),
        ];
        let parsed = parse_cli(&args).expect("CLI should parse");
        validate_cli(&parsed).expect("subcommand root should remain valid");
    }

    #[test]
    fn parse_status_ready_flag() {
        let cli = GateCli::parse_from(["gate", "status", "--ready"]);
        let Some(command) = cli.command else {
            panic!("status subcommand should be present");
        };
        let request = command.into_request().expect("status request should build");
        let CommandRequest::Status(args) = request else {
            panic!("status subcommand should produce a status request");
        };
        assert!(args.filter.ready_only);
        assert_eq!(args.filter.state, None);
        assert!(!args.filter.include_done);
    }

    #[test]
    fn parse_status_state_flag() {
        let cli = GateCli::parse_from(["gate", "status", "--state", "swe"]);
        let Some(command) = cli.command else {
            panic!("status subcommand should be present");
        };
        let request = command.into_request().expect("status request should build");
        let CommandRequest::Status(args) = request else {
            panic!("status subcommand should produce a status request");
        };
        assert_eq!(args.filter.state.as_deref(), Some("swe"));
        assert!(!args.filter.ready_only);
        assert!(!args.filter.include_done);
    }

    #[test]
    fn parse_status_all_flag() {
        let cli = GateCli::parse_from(["gate", "status", "--all"]);
        let Some(command) = cli.command else {
            panic!("status subcommand should be present");
        };
        let request = command.into_request().expect("status request should build");
        let CommandRequest::Status(args) = request else {
            panic!("status subcommand should produce a status request");
        };
        assert!(args.filter.include_done);
        assert!(!args.filter.ready_only);
        assert_eq!(args.filter.state, None);
    }
}
