use std::path::PathBuf;

use clap::{Args, CommandFactory, FromArgMatches, Parser, Subcommand, parser::ValueSource};

use crate::cli::{
    AppendArgs, CommandRequest, GateContext, HistoryArgs, OutcomeArgs, ParsedCli, StatusArgs, SyncArgs,
};
use crate::error::GateError;
use crate::status_filter::StatusFilter;

#[derive(Debug, Parser)]
#[command(name = "gate")]
#[command(about = "File-authoritative gated delivery CLI")]
pub(crate) struct GateCli {
    #[command(subcommand)]
    pub(crate) command: Option<GateSubcommand>,

    #[arg(long)]
    pub(crate) mcp: bool,

    #[arg(long)]
    pub(crate) http: bool,

    #[arg(long, default_value_t = 8081)]
    pub(crate) port: u16,

    #[arg(long = "root")]
    pub(crate) service_root: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub(crate) enum GateSubcommand {
    Status(StatusCliArgs),
    Append(AppendCliArgs),
    History(HistoryCliArgs),
    Pass(OutcomeCliArgs),
    Fail(OutcomeCliArgs),
    Cancel(OutcomeCliArgs),
    Sync(SyncCliArgs),
    Ui(UiCliArgs),
}

impl GateSubcommand {
    pub(crate) fn into_request(self) -> Result<CommandRequest, GateError> {
        Ok(match self {
            Self::Status(args) => CommandRequest::Status(StatusArgs {
                context: args.shared.into_context(args.json),
                filter: StatusFilter {
                    state: args.state,
                    ready_only: args.ready,
                    include_done: args.all,
                },
            }),
            Self::Append(args) => CommandRequest::Append(AppendArgs {
                context: args.shared.into_context(false),
                task_id: args.task_id,
                agent: args.agent,
                notes: args.notes,
                model: args.model,
            }),
            Self::History(args) => CommandRequest::History(HistoryArgs {
                context: args.shared.into_context(false),
                task_id: args.task_id,
            }),
            Self::Pass(args) => CommandRequest::Pass(args.into_outcome_args()),
            Self::Fail(args) => CommandRequest::Fail(args.into_outcome_args()),
            Self::Cancel(args) => CommandRequest::Cancel(args.into_outcome_args()),
            Self::Sync(args) => CommandRequest::Sync(SyncArgs {
                context: args.shared.into_context(false),
                rebuild: args.rebuild,
            }),
            Self::Ui(_) => {
                return Err(GateError::Cli(
                    "ui subcommand must be handled directly".to_string(),
                ));
            }
        })
    }
}

#[derive(Debug, Clone, Args)]
pub(crate) struct SharedCliArgs {
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    #[arg(long)]
    pub(crate) pipeline: Option<String>,
}

impl SharedCliArgs {
    pub(crate) fn into_context(self, json_output: bool) -> GateContext {
        GateContext {
            root: self.root,
            requested_pipeline_name: self.pipeline,
            json_output,
        }
    }
}

#[derive(Debug, Args)]
pub(crate) struct StatusCliArgs {
    #[command(flatten)]
    pub(crate) shared: SharedCliArgs,
    #[arg(long)]
    pub(crate) json: bool,
    #[arg(long)]
    pub(crate) all: bool,
    #[arg(long)]
    pub(crate) state: Option<String>,
    #[arg(long)]
    pub(crate) ready: bool,
}

#[derive(Debug, Args)]
pub(crate) struct AppendCliArgs {
    pub(crate) task_id: String,
    #[command(flatten)]
    pub(crate) shared: SharedCliArgs,
    #[arg(long)]
    pub(crate) agent: String,
    #[arg(long)]
    pub(crate) notes: String,
    #[arg(long)]
    pub(crate) model: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct HistoryCliArgs {
    pub(crate) task_id: String,
    #[command(flatten)]
    pub(crate) shared: SharedCliArgs,
}

#[derive(Debug, Args)]
pub(crate) struct OutcomeCliArgs {
    pub(crate) task_id: String,
    #[command(flatten)]
    pub(crate) shared: SharedCliArgs,
    #[arg(long)]
    pub(crate) agent: String,
    #[arg(long)]
    pub(crate) notes: String,
    #[arg(long)]
    pub(crate) model: Option<String>,
}

impl OutcomeCliArgs {
    pub(crate) fn into_outcome_args(self) -> OutcomeArgs {
        OutcomeArgs {
            context: self.shared.into_context(false),
            task_id: self.task_id,
            agent: self.agent,
            notes: self.notes,
            model: self.model,
        }
    }
}

#[derive(Debug, Args)]
pub(crate) struct SyncCliArgs {
    #[command(flatten)]
    pub(crate) shared: SharedCliArgs,
    #[arg(long)]
    pub(crate) rebuild: bool,
}

#[derive(Debug, Args)]
pub(crate) struct UiCliArgs {
    #[command(flatten)]
    pub(crate) shared: SharedCliArgs,
}

pub(crate) fn parse_cli(args: &[String]) -> Result<ParsedCli, clap::Error> {
    let argv = std::iter::once("gate".to_string()).chain(args.iter().cloned());
    let matches = GateCli::command().try_get_matches_from(argv)?;
    let port_was_explicit = matches.value_source("port") == Some(ValueSource::CommandLine);
    let cli = GateCli::from_arg_matches(&matches)?;
    Ok(ParsedCli {
        cli,
        port_was_explicit,
    })
}

pub(crate) fn validate_cli(parsed: &ParsedCli) -> Result<(), GateError> {
    if parsed.cli.mcp && parsed.cli.http {
        return Err(GateError::Cli(
            "--mcp and --http are mutually exclusive".to_string(),
        ));
    }
    if parsed.port_was_explicit && !parsed.cli.http {
        return Err(GateError::Cli(
            "--port may only be used with --http".to_string(),
        ));
    }
    if parsed.cli.service_root.is_some() && !(parsed.cli.mcp || parsed.cli.http) {
        return Err(GateError::Cli(
            "top-level --root may only be used with --mcp or --http".to_string(),
        ));
    }
    if (parsed.cli.mcp || parsed.cli.http) && parsed.cli.command.is_some() {
        return Err(GateError::Cli(
            "service modes do not accept gate subcommands".to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn usage() -> String {
    let mut command = GateCli::command();
    let mut buffer = Vec::new();
    command
        .write_long_help(&mut buffer)
        .expect("gate clap help should render");
    String::from_utf8(buffer).expect("clap help should be utf-8")
}

pub(crate) fn render_clap_error(error: clap::Error) -> i32 {
    let code = if error.use_stderr() { 1 } else { 0 };
    if error.use_stderr() {
        eprintln!("{error}");
    } else {
        println!("{error}");
    }
    code
}
