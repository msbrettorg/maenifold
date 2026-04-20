use std::path::PathBuf;

pub use maenifold_core::CliAction;

use crate::cli_parser::*;
use crate::append_command::AppendCommand;
use crate::cancel_command::CancelCommand;
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
    Cancel(OutcomeArgs),
    Sync(SyncArgs),
}

pub struct GateCliApp;

#[derive(Debug)]
pub(crate) struct ParsedCli {
    pub(crate) cli: GateCli,
    pub(crate) port_was_explicit: bool,
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
        CommandRequest::Cancel(args) => CancelCommand::execute(args),
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
