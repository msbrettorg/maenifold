use std::path::PathBuf;

pub use maenifold_core::CliAction;

use crate::append_command::AppendCommand;
use crate::cancel_command::CancelCommand;
use crate::cli_parser::*;
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
pub struct CreateArgs {
    pub context: GateContext,
    pub task_id: String,
    pub title: String,
    pub pipeline: String,
    pub priority: String,
    pub body: Option<String>,
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

pub type CancelArgs = OutcomeArgs;

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
#[path = "cli_tests.rs"]
mod tests;
