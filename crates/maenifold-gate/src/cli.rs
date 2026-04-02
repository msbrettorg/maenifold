use std::ffi::OsString;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::error::GateError;

#[derive(Debug, Clone)]
pub struct GateContext {
    pub root: PathBuf,
    pub requested_pipeline_name: Option<String>,
    pub json_output: bool,
}

#[derive(Debug, Clone)]
pub struct StatusArgs {
    pub context: GateContext,
}

#[derive(Debug, Clone)]
pub struct AppendArgs {
    pub context: GateContext,
    pub task_id: String,
    pub agent: String,
    pub notes: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VerdictArgs {
    pub context: GateContext,
    pub task_id: String,
    pub notes: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SyncArgs {
    pub context: GateContext,
    pub rebuild: bool,
}

#[derive(Debug, Clone)]
pub enum CommandRequest {
    Status(StatusArgs),
    Append(AppendArgs),
    Pass(VerdictArgs),
    Fail(VerdictArgs),
    Sync(SyncArgs),
}

#[derive(Debug)]
pub struct CliParser;

impl CliParser {
    pub fn parse<I, T>(argv: I) -> Result<CommandRequest, GateError>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let cli = GateCli::try_parse_from(argv)
            .map_err(|error| GateError::Cli(error.to_string()))?;
        Ok(cli.into_request())
    }
}

#[derive(Debug, Parser)]
#[command(name = "gate")]
#[command(about = "File-authoritative gated delivery CLI")]
struct GateCli {
    #[command(subcommand)]
    command: GateSubcommand,
}

impl GateCli {
    fn into_request(self) -> CommandRequest {
        match self.command {
            GateSubcommand::Status(args) => CommandRequest::Status(StatusArgs {
                context: args.shared.into_context(args.json),
            }),
            GateSubcommand::Append(args) => CommandRequest::Append(AppendArgs {
                context: args.shared.into_context(false),
                task_id: args.task_id,
                agent: args.agent,
                notes: args.notes,
                model: args.model,
            }),
            GateSubcommand::Pass(args) => CommandRequest::Pass(args.into_verdict_args()),
            GateSubcommand::Fail(args) => CommandRequest::Fail(args.into_verdict_args()),
            GateSubcommand::Sync(args) => CommandRequest::Sync(SyncArgs {
                context: args.shared.into_context(false),
                rebuild: args.rebuild,
            }),
        }
    }
}

#[derive(Debug, Subcommand)]
enum GateSubcommand {
    Status(StatusCliArgs),
    Append(AppendCliArgs),
    Pass(VerdictCliArgs),
    Fail(VerdictCliArgs),
    Sync(SyncCliArgs),
}

#[derive(Debug, Clone, Args)]
struct SharedCliArgs {
    #[arg(long, default_value = ".")]
    root: PathBuf,
    #[arg(long)]
    pipeline: Option<String>,
}

impl SharedCliArgs {
    fn into_context(self, json_output: bool) -> GateContext {
        GateContext {
            root: self.root,
            requested_pipeline_name: self.pipeline,
            json_output,
        }
    }
}

#[derive(Debug, Args)]
struct StatusCliArgs {
    #[command(flatten)]
    shared: SharedCliArgs,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct AppendCliArgs {
    task_id: String,
    #[command(flatten)]
    shared: SharedCliArgs,
    #[arg(long)]
    agent: String,
    #[arg(long)]
    notes: String,
    #[arg(long)]
    model: Option<String>,
}

#[derive(Debug, Args)]
struct VerdictCliArgs {
    task_id: String,
    #[command(flatten)]
    shared: SharedCliArgs,
    #[arg(long)]
    notes: String,
    #[arg(long)]
    model: Option<String>,
}

impl VerdictCliArgs {
    fn into_verdict_args(self) -> VerdictArgs {
        VerdictArgs {
            context: self.shared.into_context(false),
            task_id: self.task_id,
            notes: self.notes,
            model: self.model,
        }
    }
}

#[derive(Debug, Args)]
struct SyncCliArgs {
    #[command(flatten)]
    shared: SharedCliArgs,
    #[arg(long)]
    rebuild: bool,
}
