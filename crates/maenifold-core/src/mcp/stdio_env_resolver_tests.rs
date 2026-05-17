use std::collections::HashMap;

use crate::acp_config::EnvFromCommand;
use crate::mcp::stdio_env_resolver::resolve_mcp_server_env;

#[test]
fn mcp_server_env_from_file_resolves_literal_value() {
    let root = tempfile::tempdir().expect("create env file root");
    let key_path = root.path().join("azure-key");
    std::fs::write(&key_path, "file-key-value\n").expect("write env file");

    let env = HashMap::new();
    let mut env_from_file = HashMap::new();
    env_from_file.insert(
        "AZURE_KEY".to_string(),
        key_path.to_string_lossy().to_string(),
    );
    let env_from_command = HashMap::new();

    let resolved = resolve_mcp_server_env(&env, &env_from_file, &env_from_command)
        .expect("resolve stdio MCP env from file");

    assert_eq!(
        resolved.get("AZURE_KEY"),
        Some(&"file-key-value".to_string())
    );
}

#[test]
fn mcp_server_env_from_command_resolves_stdout() {
    let env = HashMap::new();
    let env_from_file = HashMap::new();
    let mut env_from_command = HashMap::new();
    env_from_command.insert(
        "API_KEY".to_string(),
        EnvFromCommand {
            command: "/bin/echo".to_string(),
            args: vec!["cmd-key-value".to_string()],
        },
    );

    let resolved = resolve_mcp_server_env(&env, &env_from_file, &env_from_command)
        .expect("resolve stdio MCP env from command");

    assert_eq!(resolved.get("API_KEY"), Some(&"cmd-key-value".to_string()));
}

#[test]
fn mcp_server_env_precedence_literal_over_file_and_command() {
    let root = tempfile::tempdir().expect("create env file root");
    let key_path = root.path().join("my-key");
    std::fs::write(&key_path, "file-loses\n").expect("write env file");

    let mut env = HashMap::new();
    env.insert("MY_KEY".to_string(), "literal-wins".to_string());
    let mut env_from_file = HashMap::new();
    env_from_file.insert("MY_KEY".to_string(), key_path.to_string_lossy().to_string());
    let mut env_from_command = HashMap::new();
    env_from_command.insert(
        "MY_KEY".to_string(),
        EnvFromCommand {
            command: "/bin/echo".to_string(),
            args: vec!["cmd-loses".to_string()],
        },
    );

    let resolved = resolve_mcp_server_env(&env, &env_from_file, &env_from_command)
        .expect("resolve stdio MCP env precedence");

    assert_eq!(resolved.get("MY_KEY"), Some(&"literal-wins".to_string()));
}
