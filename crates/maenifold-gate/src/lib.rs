pub mod append;
#[cfg(feature = "app")]
pub mod board;
pub mod cli;
pub mod db;
pub mod fs;
#[cfg(feature = "app")]
pub mod mcp;
pub mod parser;
pub mod pass;
pub mod pipeline;
pub mod registry;
pub mod reject;
pub mod status;
pub mod sync;
#[cfg(feature = "app")]
pub mod trace;
#[cfg(feature = "app")]
pub mod tray;
pub mod types;

pub use cli::{
    Cli, CliError, Command, CommandStubError, HelpTopic, StatusArgs, SyncArgs, parse_from, run,
    run_from,
};
#[allow(deprecated)]
pub use parser::{
    ParseTaskError, ParsedSection, extract_sections, parse_task_content, parse_task_file,
};
pub use types::{StageState, StageVerdict, TaskLocation, TaskState};
