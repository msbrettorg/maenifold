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

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match cli::GateCliApp::classify(&args) {
        cli::CliAction::Mcp => tray::run_with_tray(tray::GateServerMode::Stdio),
        cli::CliAction::Http(port) => tray::run_with_tray(tray::GateServerMode::Http(port)),
        cli::CliAction::Headless => cli::GateCliApp::run(&args),
    };
    ExitCode::from(code as u8)
}
