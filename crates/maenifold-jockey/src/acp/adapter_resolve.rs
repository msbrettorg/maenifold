use std::path::PathBuf;

use super::{
    command_parts, entry_env, env_contains, env_passthrough, find_agent_entry, validate_env,
};

pub(super) fn resolve_adapter_for_name(
    runtime: &str,
    root: &PathBuf,
) -> Result<(String, Vec<String>, Vec<(String, String)>), String> {
    let (config, entry) = find_agent_entry(root, runtime)?;
    let (binary, args) = command_parts(&entry);
    if binary.is_empty() {
        return Err(format!("agent '{}' has an empty command", runtime));
    }
    let mut env = entry_env(&entry);
    for (var_name, value) in validate_env(&entry).map_err(|error| {
        let details = error
            .failures
            .into_iter()
            .map(|failure| {
                format!(
                    "{} from {} ({})",
                    failure.var_name, failure.source, failure.reason
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        format!(
            "{} for agent '{runtime}': {details}",
            super::PREFLIGHT_ENV_VALIDATION_FAILED_PREFIX
        )
    })? {
        if !env_contains(&env, &var_name) {
            env.push((var_name, value));
        }
    }
    for var_name in env_passthrough(&config) {
        if !env_contains(&env, var_name) {
            if let Ok(value) = std::env::var(var_name) {
                env.push((var_name.to_string(), value));
            }
        }
    }
    Ok((
        binary.to_string(),
        args.into_iter().map(ToString::to_string).collect(),
        env,
    ))
}
