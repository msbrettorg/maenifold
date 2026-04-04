use crate::error::MaenifoldError;
use clap::{CommandFactory, FromArgMatches, parser::ValueSource};

use super::Cli;

pub(super) struct ParsedCli {
    pub(super) cli: Cli,
    pub(super) port_was_explicit: bool,
    pub(super) warn_deprecated_tool: bool,
}

struct NormalizedArgs {
    args: Vec<String>,
    warn_deprecated_tool: bool,
}

pub(super) fn parse_cli(args: &[String]) -> Result<ParsedCli, clap::Error> {
    let normalized = normalize_args(args);
    let argv = std::iter::once("maenifold".to_string()).chain(normalized.args.iter().cloned());
    let matches = Cli::command().try_get_matches_from(argv)?;
    let port_was_explicit = matches.value_source("port") == Some(ValueSource::CommandLine);
    let cli = Cli::from_arg_matches(&matches)?;
    Ok(ParsedCli {
        cli,
        port_was_explicit,
        warn_deprecated_tool: normalized.warn_deprecated_tool,
    })
}

pub(super) fn validate_cli(parsed: &ParsedCli) -> Result<(), String> {
    let runtime_mode_count =
        usize::from(parsed.cli.mcp) + usize::from(parsed.cli.http) + usize::from(parsed.cli.app);

    if runtime_mode_count > 1 {
        return Err("Error: --mcp, --http, and --app are mutually exclusive".to_string());
    }

    if parsed.cli.command.is_some() && runtime_mode_count > 0 {
        return Err("Error: runtime flags cannot be combined with a subcommand".to_string());
    }

    if parsed.port_was_explicit && !(parsed.cli.http || parsed.cli.app) {
        return Err("Error: --port is only valid with --http or --app".to_string());
    }

    Ok(())
}

pub(super) fn render_clap_error(err: clap::Error) -> i32 {
    let code = if matches!(
        err.kind(),
        clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
    ) {
        0
    } else {
        2
    };

    if err.use_stderr() {
        eprint!("{err}");
    } else {
        print!("{err}");
    }

    code
}

pub(super) fn render_error(
    output_format: crate::util::json::OutputFormat,
    tool_name: &str,
    err: MaenifoldError,
) -> String {
    use crate::util::json::failure;

    match output_format {
        crate::util::json::OutputFormat::Json => match err {
            MaenifoldError::UnknownTool(_) => {
                failure("UNKNOWN_TOOL", format!("Unknown tool: {tool_name}"))
            }
            MaenifoldError::InvalidArgument(msg) => failure(
                "INVALID_ARGUMENT",
                format!("Error invoking {tool_name}: {msg}"),
            ),
            MaenifoldError::MissingProperty(msg) => failure(
                "INVALID_ARGUMENT",
                format!(
                    "Error invoking {tool_name}: Required property '{msg}' is missing from payload."
                ),
            ),
            MaenifoldError::InvalidPayload(msg) => failure(
                "INVALID_PAYLOAD",
                format!("Error invoking {tool_name}: {msg}"),
            ),
            MaenifoldError::Io(ref io_err)
                if io_err.raw_os_error() == Some(63)
                    || io_err.to_string().contains("name too long") =>
            {
                failure(
                    "PATH_TOO_LONG",
                    format!("Error invoking {tool_name}: File name too long"),
                )
            }
            other => failure(
                "INTERNAL_ERROR",
                format!("Error invoking {tool_name}: {other}"),
            ),
        },
        crate::util::json::OutputFormat::Markdown => match err {
            MaenifoldError::UnknownTool(_) => format!("Unknown tool: {tool_name}"),
            other => format!("Error invoking {tool_name}: {other}"),
        },
    }
}

fn normalize_args(args: &[String]) -> NormalizedArgs {
    let Some(tool_index) = args.iter().position(|arg| arg == "--tool") else {
        return NormalizedArgs {
            args: args.to_vec(),
            warn_deprecated_tool: false,
        };
    };

    let mut normalized = vec!["tool".to_string()];
    let mut consumed = vec![tool_index];

    if let Some(tool_name) = args.get(tool_index + 1) {
        normalized.push(tool_name.clone());
        consumed.push(tool_index + 1);
    }

    if let Some(payload_index) = args.iter().position(|arg| arg == "--payload") {
        consumed.push(payload_index);
        if let Some(payload) = args.get(payload_index + 1) {
            normalized.push(payload.clone());
            consumed.push(payload_index + 1);
        }
    }

    for (index, arg) in args.iter().enumerate() {
        if !consumed.contains(&index) {
            normalized.push(arg.clone());
        }
    }

    NormalizedArgs {
        args: normalized,
        warn_deprecated_tool: true,
    }
}
