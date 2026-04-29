use super::*;

fn adapter(runtime_key: &str, binary: &str, args: &[&str]) -> jockey::acp::StdioAdapterSpec {
    jockey::acp::StdioAdapterSpec {
        runtime_key: runtime_key.to_string(),
        binary: binary.to_string(),
        args: args.iter().map(|arg| arg.to_string()).collect(),
        env: Vec::new(),
    }
}

fn adapter_with_env(
    runtime_key: &str,
    binary: &str,
    env: &[(&str, &str)],
) -> jockey::acp::StdioAdapterSpec {
    jockey::acp::StdioAdapterSpec {
        runtime_key: runtime_key.to_string(),
        binary: binary.to_string(),
        args: Vec::new(),
        env: env
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect(),
    }
}

#[test]
fn claude_caps_enable_image_not_audio() {
    let caps = caps_for_agent("claude", &adapter("claude", "npx", &["claude-code-acp"]));
    assert!(caps.image);
    assert!(!caps.audio);
    assert!(caps.embedded_context);
}

#[test]
fn gpt_5_caps_enable_image_and_audio() {
    let caps = caps_for_agent("gpt-5", &adapter("gpt-5", "codex", &["--acp"]));
    assert!(caps.image);
    assert!(caps.audio);
    assert!(caps.embedded_context);
}

#[test]
fn ollama_caps_are_conservative() {
    let caps = caps_for_agent("ollama-llama3", &adapter("ollama-llama3", "ollama", &[]));
    assert!(!caps.image);
    assert!(!caps.audio);
    assert!(caps.embedded_context);
}

#[test]
fn unknown_vendor_caps_are_conservative() {
    let caps = caps_for_agent("unknown-vendor", &adapter("unknown-vendor", "agent", &[]));
    assert!(!caps.image);
    assert!(!caps.audio);
    assert!(caps.embedded_context);
}

#[test]
fn unknown_vendor_ignores_openai_env_key_for_media_caps() {
    let caps = caps_for_agent(
        "local-agent",
        &adapter_with_env("local-agent", "node", &[("OPENAI_API_KEY", "redacted")]),
    );
    assert!(!caps.image);
    assert!(!caps.audio);
    assert!(caps.embedded_context);
}

#[test]
fn unknown_vendor_ignores_anthropic_env_key_for_media_caps() {
    let caps = caps_for_agent(
        "local-agent",
        &adapter_with_env("local-agent", "node", &[("ANTHROPIC_API_KEY", "redacted")]),
    );
    assert!(!caps.image);
    assert!(!caps.audio);
    assert!(caps.embedded_context);
}
