use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentCaps {
    pub image: bool,
    pub audio: bool,
    pub embedded_context: bool,
}

impl AgentCaps {
    fn media(image: bool, audio: bool) -> Self {
        Self {
            image,
            audio,
            embedded_context: true,
        }
    }
}

pub fn caps_for_agent(agent_name: &str, adapter: &jockey::acp::StdioAdapterSpec) -> AgentCaps {
    explicit_caps(agent_name).unwrap_or_else(|| inferred_caps(agent_name, adapter))
}

fn explicit_caps(agent_name: &str) -> Option<AgentCaps> {
    let root = jockey::acp::config_root();
    let path = root.join("maenifold.json");
    let contents = std::fs::read_to_string(path).ok()?;
    let config: Value = serde_json::from_str(&contents).ok()?;
    let entry = config
        .get("agents")?
        .as_object()?
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(agent_name))?
        .1;
    let prompt_caps = entry
        .get("promptCapabilities")
        .or_else(|| entry.get("acpPromptCapabilities"))
        .or_else(|| {
            entry
                .get("capabilities")
                .and_then(|v| v.get("promptCapabilities"))
        })?;
    Some(AgentCaps {
        image: prompt_caps.get("image").and_then(Value::as_bool)?,
        audio: prompt_caps.get("audio").and_then(Value::as_bool)?,
        embedded_context: prompt_caps
            .get("embeddedContext")
            .and_then(Value::as_bool)
            .unwrap_or(true),
    })
}

fn inferred_caps(agent_name: &str, adapter: &jockey::acp::StdioAdapterSpec) -> AgentCaps {
    let haystack = vendor_source(agent_name, adapter);
    if contains_any(&haystack, &["ollama"]) {
        return AgentCaps::media(false, false);
    }
    if contains_any(&haystack, &["claude", "anthropic"]) {
        return AgentCaps::media(true, false);
    }
    if contains_any(&haystack, &["copilot"]) {
        return AgentCaps::media(true, false);
    }
    if contains_any(&haystack, &["gpt-5", "openai", "codex", "gpt-4o"]) {
        return AgentCaps::media(true, true);
    }
    AgentCaps::media(false, false)
}

fn vendor_source(agent_name: &str, adapter: &jockey::acp::StdioAdapterSpec) -> String {
    let mut parts = vec![
        agent_name.to_ascii_lowercase(),
        adapter.runtime_key.to_ascii_lowercase(),
        adapter.binary.to_ascii_lowercase(),
    ];
    parts.extend(adapter.args.iter().map(|arg| arg.to_ascii_lowercase()));
    parts.extend(adapter.env.iter().map(|(key, _)| key.to_ascii_lowercase()));
    parts.join(" ")
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

#[cfg(test)]
#[path = "acp_relay_caps_tests.rs"]
mod tests;
