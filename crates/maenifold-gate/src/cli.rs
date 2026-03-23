use std::{env, ffi::OsString, path::PathBuf};

use crate::{append, pass, reject};

mod error;
mod help;
mod runtime;

pub use error::{CliError, CommandStubError};
pub use runtime::{run, run_from};

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

    let (mut db, mut root, mut notes, mut agent) = (None, None, None, None);
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

    let (mut db, mut root, mut json) = (None, None, false);
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

    let (mut db, mut rebuild) = (None, false);
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
    let (mut db, mut root, mut notes, mut pipeline) = (None, None, None, None);
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
