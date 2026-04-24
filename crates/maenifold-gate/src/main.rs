mod append_command;
mod cache_db;
mod cache_task_row;
mod cancel_command;
mod cli;
mod cli_parser;
mod error;
mod fail_command;
mod file_watcher;
mod gate_catalog;
mod gate_definition;
mod gate_tool_service;
mod history_command;
mod json_output;
mod mcp;
mod pass_command;
mod section_factory;
mod state_machine;
mod status_command;
mod status_filter;
mod sync_command;
mod task_file;
mod task_id;
mod task_metadata;
mod task_repository;
mod task_sections;
mod task_status;
mod tray;
mod tray_lifecycle;
mod tray_menu;
mod ui_filtering;
mod ui_progress;
mod ui_task_data;
mod ui_window;
mod ui_window_detail;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match cli::GateCliApp::classify(&args) {
        cli::CliAction::Mcp { root } => {
            tray::run_with_tray(tray_lifecycle::GateServerMode::Stdio, root)
        }
        cli::CliAction::Http { port, root } => {
            tray::run_with_tray(tray_lifecycle::GateServerMode::Http(port), root)
        }
        cli::CliAction::Headless => cli::GateCliApp::run(&args),
    };
    ExitCode::from(code as u8)
}
