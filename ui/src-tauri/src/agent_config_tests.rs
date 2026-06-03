use std::collections::HashMap;

use crate::config::{
    agent_to_daemon_value, resolve_env, AgentConfig, AgentTransport, EnvFromCommand,
};

fn stdio_config(command: Option<&str>, args: Option<Vec<&str>>) -> AgentConfig {
    AgentConfig {
        transport: AgentTransport::Stdio,
        command: command.map(str::to_string),
        args: args.map(|values| values.into_iter().map(str::to_string).collect()),
        env: HashMap::new(),
        env_from_file: HashMap::new(),
        env_from_command: HashMap::new(),
        url: None,
        headers: None,
    }
}

#[test]
fn agent_config_deserializes_env_from_file() {
    let config: AgentConfig =
        serde_json::from_str(r#"{"command":"/bin/echo","envFromFile":{"FOO":"/tmp/foo"}}"#)
            .expect("AgentConfig should deserialize");

    assert_eq!(
        config.env_from_file.get("FOO").map(String::as_str),
        Some("/tmp/foo")
    );

    let roundtrip = serde_json::to_value(&config).expect("AgentConfig should serialize");
    assert_eq!(
        roundtrip
            .get("envFromFile")
            .and_then(|value| value.get("FOO"))
            .and_then(|value| value.as_str()),
        Some("/tmp/foo")
    );
}

#[test]
fn agent_config_deserializes_env_from_command() {
    let config: AgentConfig = serde_json::from_str(
        r#"{"command":"/bin/echo","envFromCommand":{"BAR":{"command":"/bin/echo","args":["hello"]}}}"#,
    )
    .expect("AgentConfig should deserialize");

    let spec = config
        .env_from_command
        .get("BAR")
        .expect("BAR entry should deserialize");
    assert_eq!(spec.command, "/bin/echo");
    assert_eq!(spec.args, vec!["hello".to_string()]);

    let roundtrip = serde_json::to_value(&config).expect("AgentConfig should serialize");
    assert_eq!(
        roundtrip
            .get("envFromCommand")
            .and_then(|value| value.get("BAR"))
            .and_then(|value| value.get("command"))
            .and_then(|value| value.as_str()),
        Some("/bin/echo")
    );
    assert_eq!(
        roundtrip
            .get("envFromCommand")
            .and_then(|value| value.get("BAR"))
            .and_then(|value| value.get("args"))
            .and_then(|value| value.as_array())
            .and_then(|value| value.first())
            .and_then(|value| value.as_str()),
        Some("hello")
    );
}

#[test]
fn command_parts_whitespace_splits_when_args_absent() {
    let config = stdio_config(Some("/path/copilot --acp"), None);
    assert_eq!(
        config.command_parts(),
        Some(("/path/copilot".to_string(), vec!["--acp".to_string()]))
    );

    let multi = stdio_config(Some("droid exec -o acp --auto high"), None);
    assert_eq!(
        multi.command_parts(),
        Some((
            "droid".to_string(),
            vec![
                "exec".to_string(),
                "-o".to_string(),
                "acp".to_string(),
                "--auto".to_string(),
                "high".to_string(),
            ],
        ))
    );
}

#[test]
fn command_parts_uses_explicit_args_when_present() {
    let config = stdio_config(Some("/path/with spaces/bin"), Some(vec!["--flag"]));

    assert_eq!(
        config.command_parts(),
        Some((
            "/path/with spaces/bin".to_string(),
            vec!["--flag".to_string()],
        ))
    );
}

#[test]
fn command_parts_expands_tilde_in_binary() {
    let Ok(home) = std::env::var("HOME") else {
        return;
    };

    let config = stdio_config(Some("~/.local/bin/droid acp"), None);
    assert_eq!(
        config.command_parts(),
        Some((format!("{home}/.local/bin/droid"), vec!["acp".to_string()]))
    );
}

#[test]
fn command_parts_returns_none_when_command_missing() {
    let config = stdio_config(None, None);
    assert_eq!(config.command_parts(), None);
}

#[test]
fn resolve_env_layers_file_then_command_then_literal() {
    let secret_file = tempfile::NamedTempFile::new().expect("temp file");
    std::fs::write(secret_file.path(), "secret-value\n").expect("write secret");

    let mut config = stdio_config(Some("/bin/echo"), None);
    config.env_from_file.insert(
        "FILE_ONLY".to_string(),
        secret_file.path().display().to_string(),
    );
    config.env_from_file.insert(
        "LAYERED".to_string(),
        secret_file.path().display().to_string(),
    );
    config.env_from_command.insert(
        "COMMAND_ONLY".to_string(),
        EnvFromCommand {
            command: "/bin/echo".to_string(),
            args: vec!["command-value".to_string()],
        },
    );
    config.env_from_command.insert(
        "LAYERED".to_string(),
        EnvFromCommand {
            command: "/bin/echo".to_string(),
            args: vec!["command-value".to_string()],
        },
    );
    config
        .env
        .insert("LAYERED".to_string(), "literal-value".to_string());

    let resolved = resolve_env(&config).expect("env resolution should succeed");
    assert_eq!(
        resolved.get("FILE_ONLY").map(String::as_str),
        Some("secret-value")
    );
    assert_eq!(
        resolved.get("COMMAND_ONLY").map(String::as_str),
        Some("command-value")
    );
    assert_eq!(
        resolved.get("LAYERED").map(String::as_str),
        Some("literal-value")
    );
}

#[test]
fn resolve_env_returns_error_when_file_missing() {
    let mut config = stdio_config(Some("/bin/echo"), None);
    let missing_path = "/nonexistent/path/T-414-3-test";
    config
        .env_from_file
        .insert("MISSING".to_string(), missing_path.to_string());

    let error = resolve_env(&config).expect_err("missing file should error");
    assert!(
        error.contains(missing_path),
        "error should mention missing path, got: {error}"
    );
}

#[test]
fn agent_to_daemon_value_preserves_explicit_args() {
    let config = stdio_config(Some("/path with spaces/bin"), Some(vec!["--flag"]));

    let value = agent_to_daemon_value(&config);
    assert_eq!(
        value.get("command").and_then(serde_json::Value::as_str),
        Some("/path with spaces/bin")
    );
    assert_eq!(
        value.get("args").and_then(serde_json::Value::as_array),
        Some(&vec![serde_json::Value::String("--flag".to_string())])
    );
}

#[test]
fn agent_to_daemon_value_preserves_env_from_file() {
    let mut config = stdio_config(Some("/bin/codex"), Some(vec!["acp"]));
    config
        .env_from_file
        .insert("OPENAI_API_KEY".to_string(), "~/.secrets/key".to_string());

    let value = agent_to_daemon_value(&config);
    assert_eq!(
        value
            .get("envFromFile")
            .and_then(|entry| entry.get("OPENAI_API_KEY"))
            .and_then(serde_json::Value::as_str),
        Some("~/.secrets/key")
    );
}

#[test]
fn agent_to_daemon_value_preserves_env_from_command() {
    let mut config = stdio_config(Some("/bin/codex"), Some(vec!["acp"]));
    config.env_from_command.insert(
        "AZ_TOKEN".to_string(),
        EnvFromCommand {
            command: "az".to_string(),
            args: vec!["account".to_string(), "get-access-token".to_string()],
        },
    );

    let value = agent_to_daemon_value(&config);
    let command_spec = value
        .get("envFromCommand")
        .and_then(|entry| entry.get("AZ_TOKEN"))
        .expect("AZ_TOKEN entry should serialize");
    assert_eq!(
        command_spec
            .get("command")
            .and_then(serde_json::Value::as_str),
        Some("az")
    );
    assert_eq!(
        command_spec
            .get("args")
            .and_then(serde_json::Value::as_array),
        Some(&vec![
            serde_json::Value::String("account".to_string()),
            serde_json::Value::String("get-access-token".to_string())
        ])
    );
}

#[test]
fn agent_to_daemon_value_legacy_form_joins_when_args_absent() {
    let config = stdio_config(Some("/bin/foo --flag"), None);

    let value = agent_to_daemon_value(&config);
    assert_eq!(
        value.get("command").and_then(serde_json::Value::as_str),
        Some("/bin/foo --flag")
    );
    assert!(value.get("args").is_none(), "legacy form should omit args");
}

#[test]
fn roundtrip_codex_with_env_from_file() {
    let config: AgentConfig = serde_json::from_str(
        r#"{"command":"/path/codex-acp","envFromFile":{"OPENAI_API_KEY":"~/.secrets/key","AZURE_OPENAI_API_KEY":"~/.secrets/key"}}"#,
    )
    .expect("AgentConfig should deserialize");

    let value = agent_to_daemon_value(&config);
    assert_eq!(
        value.get("command").and_then(serde_json::Value::as_str),
        Some("/path/codex-acp")
    );
    let env_from_file = value
        .get("envFromFile")
        .expect("envFromFile should be preserved");
    assert_eq!(
        env_from_file
            .get("OPENAI_API_KEY")
            .and_then(serde_json::Value::as_str),
        Some("~/.secrets/key")
    );
    assert_eq!(
        env_from_file
            .get("AZURE_OPENAI_API_KEY")
            .and_then(serde_json::Value::as_str),
        Some("~/.secrets/key")
    );
}
