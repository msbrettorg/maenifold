use std::{ffi::OsString, fmt, path::PathBuf};

pub mod advance;
pub mod db;
pub mod fs;
pub mod parser;
pub mod status;
pub mod sync;
pub mod types;

pub use parser::{ParseTaskError, parse_task_content, parse_task_file};
pub use types::{StageState, StageVerdict, TaskLocation, TaskStage, TaskState, TaskStatus};

/// T-53.1: RTM FR-41 — minimal [[gate]] CLI scaffold for [[FR-41]].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cli {
    pub command: Option<Command>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Advance(AdvanceArgs),
    Status(StatusArgs),
    Sync(SyncArgs),
    Help(HelpTopic),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelpTopic {
    TopLevel,
    Advance,
    Status,
    Sync,
}

/// T-53.3: RTM FR-41 — `gate advance` contract wiring only; business logic stays thin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvanceArgs {
    pub db: Option<PathBuf>,
    pub root: Option<PathBuf>,
    pub stage: TaskStage,
    pub verdict: Option<StageVerdict>,
    pub notes: Option<String>,
    pub model: Option<String>,
    pub task: PathBuf,
}

/// T-53.1: RTM FR-41 — `gate status` keeps file-authoritative path/root/cache semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusArgs {
    pub db: Option<PathBuf>,
    pub root: Option<PathBuf>,
    pub json: bool,
    pub task: PathBuf,
}

/// T-53.1: RTM FR-41 — `gate sync` keeps root-first, cache-only semantics.
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

const TOP_LEVEL_HELP: &str = "gate\n\nUsage:\n  gate <COMMAND>\n  gate --help\n\nCommands:\n  advance   Advance a task using file-authoritative state\n  status    Report task state derived from disk\n  sync      Rebuild or reconcile the pipeline.db cache from disk\n\nNotes:\n  Markdown task files under todo/ and done/ are authoritative.\n  pipeline.db is a rebuildable cache/index only.\n";

const ADVANCE_HELP: &str = "gate-advance\n\nUsage:\n  gate advance [--db <DB>] [--root <ROOT>] --stage swe [--notes <NOTES>] [--model <MODEL>] <TASK>\n  gate advance [--db <DB>] [--root <ROOT>] --stage red --verdict PASS|REJECT [--notes <NOTES>] [--model <MODEL>] <TASK>\n  gate advance [--db <DB>] [--root <ROOT>] --stage blue --verdict PASS|FAIL [--notes <NOTES>] [--model <MODEL>] <TASK>\n  gate advance --help\n\nArguments:\n  <TASK>    Task file path to advance using file-authoritative state\n\nOptions:\n  --db <DB>            Path to the cache database (not authoritative state)\n  --root <ROOT>        Optional pipeline root containing todo/ and done/\n  --stage <STAGE>      Required stage selector: swe, red, or blue\n  --verdict <VERDICT>  Required for red/blue, forbidden for swe\n  --notes <NOTES>      Notes to append to the canonical stage section\n  --model <MODEL>      Agent model to record in the cache work_log\n";

const STATUS_HELP: &str = "gate-status\n\nUsage:\n  gate status [--db <DB>] [--root <ROOT>] [--json] [TASK]\n  gate status --help\n\nArguments:\n  [TASK]   Optional task file name to inspect; omit to scan all tasks\n\nOptions:\n  --db <DB>      Path to the cache database (not authoritative state)\n  --root <ROOT>  Optional pipeline root containing todo/ and done/\n  --json         Emit JSON instead of human-readable text\n";

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

    match first.as_str() {
        "-h" | "--help" => Ok(Cli {
            command: Some(Command::Help(HelpTopic::TopLevel)),
        }),
        "advance" => parse_advance(args.collect()).map(|command| Cli {
            command: Some(command),
        }),
        "status" => parse_status(args.collect()).map(|command| Cli {
            command: Some(command),
        }),
        "sync" => parse_sync(args.collect()).map(|command| Cli {
            command: Some(command),
        }),
        other => Err(CliError::new(format!(
            "unknown command '{other}'. Run 'gate --help' for usage."
        ))),
    }
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
        Some(Command::Help(HelpTopic::Advance)) => {
            print!("{ADVANCE_HELP}");
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
        Some(Command::Advance(args)) => run_stubbed_command(advance::run(&args)),
        Some(Command::Status(args)) => run_stubbed_command(status::run(&args)),
        Some(Command::Sync(args)) => run_stubbed_command(sync::run(&args)),
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

fn parse_advance(args: Vec<String>) -> Result<Command, CliError> {
    if contains_help(&args) {
        return Ok(Command::Help(HelpTopic::Advance));
    }

    let mut db = None;
    let mut root = None;
    let mut stage = None;
    let mut verdict = None;
    let mut notes = None;
    let mut model = None;
    let mut positional = Vec::new();

    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--db" => set_once_path(&mut db, "--db", next_value(&mut iter, "--db")?)?,
            "--root" => set_once_path(&mut root, "--root", next_value(&mut iter, "--root")?)?,
            "--stage" => set_once_stage(
                &mut stage,
                parse_stage_value(&next_value(&mut iter, "--stage")?)?,
            )?,
            "--verdict" => set_once_verdict(
                &mut verdict,
                parse_verdict_value(&next_value(&mut iter, "--verdict")?)?,
            )?,
            "--notes" => set_once_string(&mut notes, "--notes", next_value(&mut iter, "--notes")?)?,
            "--model" => set_once_string(&mut model, "--model", next_value(&mut iter, "--model")?)?,
            other if other.starts_with('-') => {
                return Err(CliError::new(format!("unknown option '{other}'")));
            }
            value => positional.push(PathBuf::from(value)),
        }
    }

    let task = only_positional("TASK", positional)?;
    let stage = stage.ok_or_else(|| CliError::new("missing required option --stage"))?;

    validate_advance_contract(stage, verdict)?;

    Ok(Command::Advance(AdvanceArgs {
        db,
        root,
        stage,
        verdict,
        notes,
        model,
        task,
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
            value => positional.push(PathBuf::from(value)),
        }
    }
    if positional.len() > 1 {
        return Err(CliError::new(
            "unexpected extra positional arguments for [TASK]",
        ));
    }
    Ok(Command::Status(StatusArgs {
        db,
        root,
        json,
        task: positional.into_iter().next().unwrap_or_default(),
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

    let root = only_positional("ROOT", positional)?;
    Ok(Command::Sync(SyncArgs { db, root, rebuild }))
}

pub(crate) fn validate_advance_contract(
    stage: TaskStage,
    verdict: Option<StageVerdict>,
) -> Result<(), CliError> {
    match (stage, verdict) {
        (TaskStage::Swe, None) => Ok(()),
        (TaskStage::Swe, Some(_)) => {
            Err(CliError::new("--verdict is not allowed when --stage swe"))
        }
        (TaskStage::Red, Some(StageVerdict::Pass | StageVerdict::Reject)) => Ok(()),
        (TaskStage::Red, Some(StageVerdict::Fail)) => {
            Err(CliError::new("--stage red requires --verdict PASS|REJECT"))
        }
        (TaskStage::Red, None) => Err(CliError::new("--verdict is required when --stage red")),
        (TaskStage::Blue, Some(StageVerdict::Pass | StageVerdict::Fail)) => Ok(()),
        (TaskStage::Blue, Some(StageVerdict::Reject)) => {
            Err(CliError::new("--stage blue requires --verdict PASS|FAIL"))
        }
        (TaskStage::Blue, None) => Err(CliError::new("--verdict is required when --stage blue")),
    }
}

fn parse_stage_value(value: &str) -> Result<TaskStage, CliError> {
    match value {
        "swe" => Ok(TaskStage::Swe),
        "red" => Ok(TaskStage::Red),
        "blue" => Ok(TaskStage::Blue),
        _ => Err(CliError::new(format!(
            "invalid value '{value}' for --stage; expected swe, red, or blue"
        ))),
    }
}

fn parse_verdict_value(value: &str) -> Result<StageVerdict, CliError> {
    let normalized = value.to_ascii_uppercase();

    match normalized.as_str() {
        "PASS" => Ok(StageVerdict::Pass),
        "REJECT" => Ok(StageVerdict::Reject),
        "FAIL" => Ok(StageVerdict::Fail),
        _ => Err(CliError::new(format!(
            "invalid value '{value}' for --verdict; expected PASS, REJECT, or FAIL"
        ))),
    }
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

fn set_once_stage(slot: &mut Option<TaskStage>, value: TaskStage) -> Result<(), CliError> {
    if slot.is_some() {
        return Err(CliError::new("duplicate option '--stage'"));
    }

    *slot = Some(value);
    Ok(())
}

fn set_once_verdict(slot: &mut Option<StageVerdict>, value: StageVerdict) -> Result<(), CliError> {
    if slot.is_some() {
        return Err(CliError::new("duplicate option '--verdict'"));
    }

    *slot = Some(value);
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

fn only_positional(name: &str, mut positional: Vec<PathBuf>) -> Result<PathBuf, CliError> {
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

#[cfg(test)]
mod tests {
    use super::{AdvanceArgs, Command, HelpTopic, StageVerdict, SyncArgs, TaskStage, parse_from};
    use std::path::PathBuf;

    #[test]
    fn advance_accepts_lowercase_red_pass_verdict() {
        let cli = parse_from([
            "gate",
            "advance",
            "--root",
            ".",
            "--stage",
            "red",
            "--verdict",
            "pass",
            "T-1.1.md",
        ])
        .unwrap();
        assert_eq!(
            cli.command,
            Some(Command::Advance(AdvanceArgs {
                db: None,
                root: Some(PathBuf::from(".")),
                stage: TaskStage::Red,
                verdict: Some(StageVerdict::Pass),
                notes: None,
                model: None,
                task: PathBuf::from("T-1.1.md"),
            }))
        );
    }

    #[test]
    fn advance_preserves_uppercase_red_reject_verdict() {
        let cli = parse_from([
            "gate",
            "advance",
            "--root",
            ".",
            "--stage",
            "red",
            "--verdict",
            "REJECT",
            "T-1.1.md",
        ])
        .unwrap();
        assert_eq!(
            cli.command,
            Some(Command::Advance(AdvanceArgs {
                db: None,
                root: Some(PathBuf::from(".")),
                stage: TaskStage::Red,
                verdict: Some(StageVerdict::Reject),
                notes: None,
                model: None,
                task: PathBuf::from("T-1.1.md"),
            }))
        );
    }

    #[test]
    fn advance_accepts_mixed_case_blue_fail_verdict() {
        let cli = parse_from([
            "gate",
            "advance",
            "--root",
            ".",
            "--stage",
            "blue",
            "--verdict",
            "Fail",
            "T-1.1.md",
        ])
        .unwrap();
        assert_eq!(
            cli.command,
            Some(Command::Advance(AdvanceArgs {
                db: None,
                root: Some(PathBuf::from(".")),
                stage: TaskStage::Blue,
                verdict: Some(StageVerdict::Fail),
                notes: None,
                model: None,
                task: PathBuf::from("T-1.1.md"),
            }))
        );
    }

    #[test]
    fn advance_rejects_invalid_verdict_value() {
        let error = parse_from([
            "gate",
            "advance",
            "--root",
            ".",
            "--stage",
            "red",
            "--verdict",
            "maybe",
            "T-1.1.md",
        ])
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "invalid value 'maybe' for --verdict; expected PASS, REJECT, or FAIL"
        );
    }

    #[test]
    fn sync_help_is_available() {
        let cli = parse_from(["gate", "sync", "--help"]).unwrap();
        assert_eq!(cli.command, Some(Command::Help(HelpTopic::Sync)));
    }

    #[test]
    fn sync_requires_root_positional() {
        let error = parse_from(["gate", "sync", "--rebuild"]).unwrap_err();
        assert_eq!(error.to_string(), "missing required argument <ROOT>");
    }

    #[test]
    fn sync_parses_root_and_rebuild_flag() {
        let cli = parse_from(["gate", "sync", "--rebuild", "."]).unwrap();
        assert_eq!(
            cli.command,
            Some(Command::Sync(SyncArgs {
                db: None,
                root: PathBuf::from("."),
                rebuild: true,
            }))
        );
    }
}
