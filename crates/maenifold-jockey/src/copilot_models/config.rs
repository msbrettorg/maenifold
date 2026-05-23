use std::{fs, path::Path};

pub(super) fn load_copilot_enterprise_uri(config_root: &Path) -> Option<String> {
    let path = config_root.join("maenifold.json");
    let contents = fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&contents).ok()?;
    let agents = value.get("agents")?.as_object()?;
    let entry = agents
        .iter()
        .find(|(name, agent)| {
            name.eq_ignore_ascii_case("copilot")
                || agent
                    .get("command")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|command| command.to_ascii_lowercase().contains("copilot"))
        })?
        .1;
    let env = entry.get("env")?.as_object()?;
    ["GITHUB_ENTERPRISE_URI", "COPILOT_ENTERPRISE_URI", "GITHUB_HOST"]
        .into_iter()
        .find_map(|key| {
            env.get(key)
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        })
}
