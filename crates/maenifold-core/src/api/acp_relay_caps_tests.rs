use super::*;

fn adapter(runtime_key: &str, binary: &str, args: &[&str]) -> jockey::acp::StdioAdapterSpec {
    jockey::acp::StdioAdapterSpec {
        runtime_key: runtime_key.to_string(),
        binary: binary.to_string(),
        args: args.iter().map(|arg| arg.to_string()).collect(),
        env: Vec::new(),
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
