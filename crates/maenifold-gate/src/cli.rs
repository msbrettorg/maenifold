use std::{env, ffi::OsString, path::PathBuf};

use crate::{append, pass, reject};

mod error;
mod help;
mod runtime;
mod util;

pub use error::{CliError, CommandStubError};
pub use runtime::{run, run_from};

use util::{
    contains_help, next_value, only_positional, only_positional_path, set_once_path,
    set_once_string,
};

pub struct Cli {
    pub command: Option<Command>,
}

pub enum Command {
    Pass(pass::PassArgs),
    Reject(reject::RejectArgs),
    Append(append::AppendArgs),
    Status(StatusArgs),
    Sync(SyncArgs),
    #[cfg(feature = "app")]
    Mcp,
    #[cfg(feature = "app")]
    Http(u16),
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
    pub pipeline_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncArgs {
    pub db: Option<PathBuf>,
    pub root: PathBuf,
    pub rebuild: bool,
    pub pipeline_path: Option<PathBuf>,
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
        #[cfg(feature = "app")]
        "--mcp" => parse_mcp(args.collect())?,
        #[cfg(feature = "app")]
        "--http" => parse_http(args.collect())?,
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

#[cfg(feature = "app")]
fn parse_mcp(args: Vec<String>) -> Result<Command, CliError> {
    if args.is_empty() {
        return Ok(Command::Mcp);
    }
    Err(CliError::new("unexpected extra arguments for --mcp"))
}

#[cfg(feature = "app")]
fn parse_http(args: Vec<String>) -> Result<Command, CliError> {
    match args.as_slice() {
        [] => Ok(Command::Http(9879)),
        [port] => port
            .parse::<u16>()
            .map(Command::Http)
            .map_err(|_| CliError::new(format!("invalid port '{port}'"))),
        _ => Err(CliError::new(
            "unexpected extra arguments for --http [PORT]",
        )),
    }
}

fn parse_pass(args: Vec<String>) -> Result<Command, CliError> {
    if contains_help(&args) {
        return Ok(Command::Help(HelpTopic::Pass));
    }
    let args = parse_transition_args(args)?;
    Ok(Command::Pass(pass::PassArgs {
        db_path: args.db.unwrap_or_else(|| args.root.join("gate.db")),
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
        db_path: args.db.unwrap_or_else(|| args.root.join("gate.db")),
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
        db_path: db.unwrap_or_else(|| root.join("gate.db")),
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

    let (mut db, mut root, mut pipeline_path, mut json) = (None, None, None, false);
    let mut positional = Vec::new();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--db" => set_once_path(&mut db, "--db", next_value(&mut iter, "--db")?)?,
            "--root" => set_once_path(&mut root, "--root", next_value(&mut iter, "--root")?)?,
            "--pipeline" => set_once_path(
                &mut pipeline_path,
                "--pipeline",
                next_value(&mut iter, "--pipeline")?,
            )?,
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
        pipeline_path,
    }))
}

fn parse_sync(args: Vec<String>) -> Result<Command, CliError> {
    if contains_help(&args) {
        return Ok(Command::Help(HelpTopic::Sync));
    }

    let (mut db, mut pipeline_path, mut rebuild) = (None, None, false);
    let mut positional = Vec::new();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--db" => set_once_path(&mut db, "--db", next_value(&mut iter, "--db")?)?,
            "--pipeline" => set_once_path(
                &mut pipeline_path,
                "--pipeline",
                next_value(&mut iter, "--pipeline")?,
            )?,
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
        pipeline_path,
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
