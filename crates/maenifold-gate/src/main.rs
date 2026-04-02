mod append_command;
mod cache_db;
mod cli;
mod error;
mod fail_command;
mod gate_catalog;
mod gate_definition;
mod json_output;
mod pass_command;
mod prd_parser;
mod reject_command;
mod rtm_parser;
mod section_factory;
mod state_machine;
mod status_command;
mod sync_command;
mod task_file;
mod task_metadata;
mod task_repository;
mod task_sections;
mod task_status;

use std::process::ExitCode;

use cli::CommandRequest;
use error::GateError;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), GateError> {
    match cli::CliParser::parse(std::env::args_os())? {
        CommandRequest::Status(args) => status_command::StatusCommand::execute(args),
        CommandRequest::Append(args) => append_command::AppendCommand::execute(args),
        CommandRequest::Pass(args) => pass_command::PassCommand::execute(args),
        CommandRequest::Reject(args) => reject_command::RejectCommand::execute(args),
        CommandRequest::Fail(args) => fail_command::FailCommand::execute(args),
        CommandRequest::Sync(args) => sync_command::SyncCommand::execute(args),
    }
}
