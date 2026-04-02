mod append_command;
mod cache_db;
mod cli;
mod error;
mod fail_command;
mod gate_catalog;
mod gate_definition;
mod gate_tool_service;
mod json_output;
mod mcp;
mod pass_command;
mod prd_parser;
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
mod tray;
mod tray_menu;

use std::process::ExitCode;

use cli::CommandRequest;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match cli::GateCliApp::classify(&args) {
        cli::CliAction::Mcp => tray::run_with_tray(tray::GateServerMode::Stdio),
        cli::CliAction::Http(port) => tray::run_with_tray(tray::GateServerMode::Http(port)),
        cli::CliAction::Headless => cli::GateCliApp::run(&args),
    };
    ExitCode::from(code as u8)
}

pub(crate) fn run_command(request: CommandRequest) -> i32 {
    let result = match request {
        CommandRequest::Status(args) => status_command::StatusCommand::execute(args),
        CommandRequest::Append(args) => append_command::AppendCommand::execute(args),
        CommandRequest::Pass(args) => pass_command::PassCommand::execute(args),
        CommandRequest::Fail(args) => fail_command::FailCommand::execute(args),
        CommandRequest::Sync(args) => sync_command::SyncCommand::execute(args),
    };

    match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("{error}");
            1
        }
    }
}
