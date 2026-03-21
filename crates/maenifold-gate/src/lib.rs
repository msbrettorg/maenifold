use std::{env, ffi::OsString, fmt, path::PathBuf};

pub mod append;
pub mod db;
pub mod fs;
pub mod parser;
pub mod pass;
pub mod pipeline;
pub mod reject;
pub mod status;
pub mod sync;
pub mod types;

pub use parser::{ParseTaskError, parse_task_content, parse_task_file};
pub use types::{StageState, StageVerdict, TaskLocation, TaskStage, TaskState, TaskStatus};

pub struct Cli {
    pub command: Option<Command>,
}

pub enum Command {
    Pass(pass::PassArgs),
    Reject(reject::RejectArgs),
    Append(append::AppendArgs),
    Status(StatusArgs),
    Sync(SyncArgs),
    Help(HelpTopic),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpTopic {
    TopLevel,
    Pass,
    Reject,
    Append,
    Status,
    Sync,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusArgs {
    pub db: Option<PathBuf>,
    pub root: PathBuf,
    pub json: bool,
    pub id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncArgs {
    pub db: Option<PathBuf>,
    pub root: PathBuf,
    pub rebuild: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliError {
    message: String,
}

impl CliError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandStubError {
    message: String,
}

impl CommandStubError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for CommandStubError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

const TOP_LEVEL_HELP: &str = "gate\n\nUsage:\n  gate <COMMAND>\n  gate --help\n\nCommands:\n  pass      Advance a task by ID using the pipeline pass transition\n  reject    Reject a task by ID using the pipeline reject transition\n  append    Append agent notes to a task by ID\n  status    Report task state derived from disk\n  sync      Rebuild or reconcile the pipeline.db cache from disk\n\nNotes:\n  Markdown task files under todo/ and done/ are authoritative.\n  pipeline.db is a rebuildable cache/index only.\n";

const PASS_HELP: &str = "gate-pass\n\nUsage:\n  gate pass [--db <DB>] [--root <ROOT>] [--pipeline <PATH>] [--notes <NOTES>] <ID>\n  gate pass --help\n\nArguments:\n  <ID>              Task ID to advance via the pipeline pass transition\n\nOptions:\n  --db <DB>         Path to the cache database (defaults to <ROOT>/pipeline.db)\n  --root <ROOT>     Pipeline root containing todo/ and done/ (defaults to current directory)\n  --pipeline <PATH> Optional pipeline definition path\n  --notes <NOTES>   Notes to append to the orchestrator section\n";

const REJECT_HELP: &str = "gate-reject\n\nUsage:\n  gate reject [--db <DB>] [--root <ROOT>] [--pipeline <PATH>] [--notes <NOTES>] <ID>\n  gate reject --help\n\nArguments:\n  <ID>              Task ID to reject via the pipeline reject transition\n\nOptions:\n  --db <DB>         Path to the cache database (defaults to <ROOT>/pipeline.db)\n  --root <ROOT>     Pipeline root containing todo/ and done/ (defaults to current directory)\n  --pipeline <PATH> Optional pipeline definition path\n  --notes <NOTES>   Notes to append to the orchestrator section\n";

const APPEND_HELP: &str = "gate-append\n\nUsage:\n  gate append [--db <DB>] [--root <ROOT>] --notes <NOTES> --agent <ROLE> <ID>\n  gate append --help\n\nArguments:\n  <ID>              Task ID to append notes to\n\nOptions:\n  --db <DB>         Path to the cache database (defaults to <ROOT>/pipeline.db)\n  --root <ROOT>     Pipeline root containing todo/ and done/ (defaults to current directory)\n  --notes <NOTES>   Required notes to append\n  --agent <ROLE>    Required agent role for the appended section heading\n";

const STATUS_HELP: &str = "gate-status\n\nUsage:\n  gate status [--db <DB>] [--root <ROOT>] [--json] [ID]\n  gate status --help\n\nArguments:\n  [ID]         Optional task ID to inspect; omit to scan all tasks\n\nOptions:\n  --db <DB>      Path to the cache database used for drift checks and ID lookup\n  --root <ROOT>  Pipeline root containing todo/ and done/ (defaults to current directory)\n  --json         Emit JSON instead of human-readable text\n";

const SYNC_HELP: &str = "gate-sync\n\nUsage:\n  gate sync [--db <DB>] [--rebuild] <ROOT>\n  gate sync --help\n\nArguments:\n  <ROOT>         Root containing authoritative todo/ and done/ task files\n\nOptions:\n  --db <DB>      Path to the cache database to rebuild/reconcile\n  --rebuild      Delete existing cached task/work log rows before inserting disk state\n";

pub fn parse_from<I, T>(args: I) -> Result<Cli, CliError>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let mut args = args
        .into_iter()
        .map(|arg| arg.into().to_string_lossy().into_owned());

    let _bin = args.next();
    let Some(first) = args.next() else {
        return Ok(Cli { command: None });
    };

    let command = match first.as_str() {
        "-h" | "--help" => Command::Help(HelpTopic::TopLevel),
        "pass" => parse_pass(args.collect())?,
        "reject" => parse_reject(args.collect())?,
        "append" => parse_append(args.collect())?,
        "status" => parse_status(args.collect())?,
        "sync" => parse_sync(args.collect())?,
        other => {
            return Err(CliError::new(format!(
                "unknown command '{other}'. Run 'gate --help' for usage."
            )));
        }
    };

    Ok(Cli {
        command: Some(command),
    })
}

pub fn run_from<I, T>(args: I) -> i32
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    match parse_from(args) {
        Ok(cli) => run(cli),
        Err(error) => {
            eprintln!("{}", error.message);
            2
        }
    }
}

pub fn run(cli: Cli) -> i32 {
    match cli.command {
        None | Some(Command::Help(HelpTopic::TopLevel)) => {
            print!("{TOP_LEVEL_HELP}");
            0
        }
        Some(Command::Help(HelpTopic::Pass)) => {
            print!("{PASS_HELP}");
            0
        }
        Some(Command::Help(HelpTopic::Reject)) => {
            print!("{REJECT_HELP}");
            0
        }
        Some(Command::Help(HelpTopic::Append)) => {
            print!("{APPEND_HELP}");
            0
        }
        Some(Command::Help(HelpTopic::Status)) => {
            print!("{STATUS_HELP}");
            0
        }
        Some(Command::Help(HelpTopic::Sync)) => {
            print!("{SYNC_HELP}");
            0
        }
        Some(Command::Pass(args)) => run_anyhow_command(pass::run(&args)),
        Some(Command::Reject(args)) => run_anyhow_command(reject::run(&args)),
        Some(Command::Append(args)) => run_anyhow_command(append::run(&args)),
        Some(Command::Status(args)) => run_stubbed_command(status::run(&args)),
        Some(Command::Sync(args)) => run_stubbed_command(sync::run(&args)),
    }
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
            eprintln!("{}", error.message);
            1
        }
    }
}

fn parse_pass(args: Vec<String>) -> Result<Command, CliError> {
    if contains_help(&args) {
        return Ok(Command::Help(HelpTopic::Pass));
    }
    let args = parse_transition_args(args)?;
    Ok(Command::Pass(pass::PassArgs {
        db_path: args.db.unwrap_or_else(|| args.root.join("pipeline.db")),
        id: args.id,
        notes: args.notes,
        pipeline_path: args.pipeline,
        root: args.root,
    }))
}

fn parse_reject(args: Vec<String>) -> Result<Command, CliError> {
    if contains_help(&args) {
        return Ok(Command::Help(HelpTopic::Reject));
    }
    let args = parse_transition_args(args)?;
    Ok(Command::Reject(reject::RejectArgs {
        db_path: args.db.unwrap_or_else(|| args.root.join("pipeline.db")),
        id: args.id,
        notes: args.notes,
        pipeline_path: args.pipeline,
        root: args.root,
    }))
}

fn parse_append(args: Vec<String>) -> Result<Command, CliError> {
    if contains_help(&args) {
        return Ok(Command::Help(HelpTopic::Append));
    }

    let mut db = None;
    let mut root = None;
    let mut notes = None;
    let mut agent = None;
    let mut positional = Vec::new();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--db" => set_once_path(&mut db, "--db", next_value(&mut iter, "--db")?)?,
            "--root" => set_once_path(&mut root, "--root", next_value(&mut iter, "--root")?)?,
            "--notes" => set_once_string(&mut notes, "--notes", next_value(&mut iter, "--notes")?)?,
            "--agent" => set_once_string(&mut agent, "--agent", next_value(&mut iter, "--agent")?)?,
            other if other.starts_with('-') => {
                return Err(CliError::new(format!("unknown option '{other}'")));
            }
            value => positional.push(value.to_owned()),
        }
    }

    let root = resolve_root(root)?;
    Ok(Command::Append(append::AppendArgs {
        db_path: db.unwrap_or_else(|| root.join("pipeline.db")),
        id: only_positional("ID", positional)?,
        notes: notes.ok_or_else(|| CliError::new("missing required option --notes"))?,
        agent: agent.ok_or_else(|| CliError::new("missing required option --agent"))?,
        root,
    }))
}

fn parse_status(args: Vec<String>) -> Result<Command, CliError> {
    if contains_help(&args) {
        return Ok(Command::Help(HelpTopic::Status));
    }

    let mut db = None;
    let mut root = None;
    let mut json = false;
    let mut positional = Vec::new();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--db" => set_once_path(&mut db, "--db", next_value(&mut iter, "--db")?)?,
            "--root" => set_once_path(&mut root, "--root", next_value(&mut iter, "--root")?)?,
            "--json" => json = true,
            other if other.starts_with('-') => {
                return Err(CliError::new(format!("unknown option '{other}'")));
            }
            value => positional.push(value.to_owned()),
        }
    }

    if positional.len() > 1 {
        return Err(CliError::new(
            "unexpected extra positional arguments for [ID]",
        ));
    }

    Ok(Command::Status(StatusArgs {
        db,
        root: resolve_root(root)?,
        json,
        id: positional.into_iter().next(),
    }))
}

fn parse_sync(args: Vec<String>) -> Result<Command, CliError> {
    if contains_help(&args) {
        return Ok(Command::Help(HelpTopic::Sync));
    }

    let mut db = None;
    let mut rebuild = false;
    let mut positional = Vec::new();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--db" => set_once_path(&mut db, "--db", next_value(&mut iter, "--db")?)?,
            "--rebuild" => rebuild = true,
            other if other.starts_with('-') => {
                return Err(CliError::new(format!("unknown option '{other}'")));
            }
            value => positional.push(PathBuf::from(value)),
        }
    }

    Ok(Command::Sync(SyncArgs {
        db,
        root: only_positional_path("ROOT", positional)?,
        rebuild,
    }))
}

struct TransitionArgs {
    id: String,
    notes: Option<String>,
    pipeline: Option<PathBuf>,
    db: Option<PathBuf>,
    root: PathBuf,
}

fn parse_transition_args(args: Vec<String>) -> Result<TransitionArgs, CliError> {
    let mut db = None;
    let mut root = None;
    let mut notes = None;
    let mut pipeline = None;
    let mut positional = Vec::new();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--db" => set_once_path(&mut db, "--db", next_value(&mut iter, "--db")?)?,
            "--root" => set_once_path(&mut root, "--root", next_value(&mut iter, "--root")?)?,
            "--notes" => set_once_string(&mut notes, "--notes", next_value(&mut iter, "--notes")?)?,
            "--pipeline" => set_once_path(
                &mut pipeline,
                "--pipeline",
                next_value(&mut iter, "--pipeline")?,
            )?,
            other if other.starts_with('-') => {
                return Err(CliError::new(format!("unknown option '{other}'")));
            }
            value => positional.push(value.to_owned()),
        }
    }

    Ok(TransitionArgs {
        id: only_positional("ID", positional)?,
        notes,
        pipeline,
        db,
        root: resolve_root(root)?,
    })
}

fn resolve_root(root: Option<PathBuf>) -> Result<PathBuf, CliError> {
    root.map(Ok).unwrap_or_else(|| {
        env::current_dir()
            .map_err(|error| CliError::new(format!("failed to resolve current directory: {error}")))
    })
}

fn set_once_path(
    slot: &mut Option<PathBuf>,
    option_name: &str,
    value: String,
) -> Result<(), CliError> {
    if slot.is_some() {
        return Err(CliError::new(format!("duplicate option '{option_name}'")));
    }
    *slot = Some(PathBuf::from(value));
    Ok(())
}

fn set_once_string(
    slot: &mut Option<String>,
    option_name: &str,
    value: String,
) -> Result<(), CliError> {
    if slot.is_some() {
        return Err(CliError::new(format!("duplicate option '{option_name}'")));
    }
    *slot = Some(value);
    Ok(())
}

fn next_value<I>(iter: &mut I, option_name: &str) -> Result<String, CliError>
where
    I: Iterator<Item = String>,
{
    iter.next()
        .ok_or_else(|| CliError::new(format!("missing value for {option_name}")))
}

fn contains_help(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "-h" || arg == "--help")
}

fn only_positional(name: &str, mut positional: Vec<String>) -> Result<String, CliError> {
    if positional.is_empty() {
        return Err(CliError::new(format!("missing required argument <{name}>")));
    }
    if positional.len() > 1 {
        return Err(CliError::new(format!(
            "unexpected extra positional arguments for <{name}>"
        )));
    }
    Ok(positional.remove(0))
}

fn only_positional_path(name: &str, positional: Vec<PathBuf>) -> Result<PathBuf, CliError> {
    only_positional(
        name,
        positional
            .into_iter()
            .map(|path| path.display().to_string())
            .collect(),
    )
    .map(PathBuf::from)
}
