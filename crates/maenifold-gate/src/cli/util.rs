use std::path::PathBuf;

use super::error::CliError;

pub(super) fn set_once_path(
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

pub(super) fn set_once_string(
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

pub(super) fn next_value<I>(iter: &mut I, option_name: &str) -> Result<String, CliError>
where
    I: Iterator<Item = String>,
{
    iter.next()
        .ok_or_else(|| CliError::new(format!("missing value for {option_name}")))
}

pub(super) fn contains_help(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "-h" || arg == "--help")
}

pub(super) fn only_positional(name: &str, mut positional: Vec<String>) -> Result<String, CliError> {
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

pub(super) fn only_positional_path(
    name: &str,
    positional: Vec<PathBuf>,
) -> Result<PathBuf, CliError> {
    only_positional(
        name,
        positional
            .into_iter()
            .map(|path| path.display().to_string())
            .collect(),
    )
    .map(PathBuf::from)
}
