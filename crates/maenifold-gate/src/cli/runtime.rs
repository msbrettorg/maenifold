use std::ffi::OsString;

use crate::{append, pass, reject, status, sync};

use super::{
    Cli, Command, CommandStubError, HelpTopic,
    help::{APPEND_HELP, PASS_HELP, REJECT_HELP, STATUS_HELP, SYNC_HELP, TOP_LEVEL_HELP},
    parse_from,
};

pub fn run_from<I, T>(args: I) -> i32
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    match parse_from(args) {
        Ok(cli) => run(cli),
        Err(error) => {
            eprintln!("{error}");
            2
        }
    }
}

pub fn run(cli: Cli) -> i32 {
    match cli.command {
        None | Some(Command::Help(HelpTopic::TopLevel)) => print_help(TOP_LEVEL_HELP),
        Some(Command::Help(HelpTopic::Pass)) => print_help(PASS_HELP),
        Some(Command::Help(HelpTopic::Reject)) => print_help(REJECT_HELP),
        Some(Command::Help(HelpTopic::Append)) => print_help(APPEND_HELP),
        Some(Command::Help(HelpTopic::Status)) => print_help(STATUS_HELP),
        Some(Command::Help(HelpTopic::Sync)) => print_help(SYNC_HELP),
        Some(Command::Pass(args)) => run_anyhow_command(pass::run(&args)),
        Some(Command::Reject(args)) => run_anyhow_command(reject::run(&args)),
        Some(Command::Append(args)) => run_anyhow_command(append::run(&args)),
        Some(Command::Status(args)) => run_stubbed_command(status::run(&args)),
        Some(Command::Sync(args)) => run_stubbed_command(sync::run(&args)),
    }
}

fn print_help(help: &str) -> i32 {
    print!("{help}");
    0
}

fn run_anyhow_command(result: anyhow::Result<()>) -> i32 {
    match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("{error}");
            1
        }
    }
}

fn run_stubbed_command(result: Result<(), CommandStubError>) -> i32 {
    match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("{error}");
            1
        }
    }
}
