use super::*;
use std::collections::HashMap;

#[test]
fn agents_config_load_or_default_defaults_when_config_missing() {
    let root = tempfile::tempdir().expect("create temp root");

    let config = AgentsConfig::load_or_default(root.path()).expect("missing config should default");

    assert!(config.agents.is_empty());
    assert!(config.mcp_servers.is_empty());
    assert!(config.settings.is_none());
}

#[test]
fn agents_config_load_or_default_defaults_when_config_unparsable() {
    let root = tempfile::tempdir().expect("create temp root");
    std::fs::write(root.path().join("maenifold.json"), "{ invalid json")
        .expect("write invalid config");

    let config =
        AgentsConfig::load_or_default(root.path()).expect("unparsable config should default");

    assert!(config.agents.is_empty());
    assert!(config.mcp_servers.is_empty());
    assert!(config.settings.is_none());
}

#[test]
fn agents_config_load_or_default_allows_intent_resolver_without_agent() {
    let root = tempfile::tempdir().expect("create temp root");
    std::fs::write(
        root.path().join("maenifold.json"),
        r#"{
            "agents": {},
            "settings": {
                "gateway": {
                    "intentResolver": {
                        "enabled": true
                    }
                }
            }
        }"#,
    )
    .expect("write config");

    let config = AgentsConfig::load_or_default(root.path())
        .expect("intent resolver config should load without an agent");

    assert!(
        config
            .settings
            .expect("settings")
            .gateway
            .intent_resolver
            .enabled
    );
}

#[test]
fn mcp_servers_schema_load_rejects_legacy_array_form_with_migration_error() {
    let root = tempfile::tempdir().expect("create temp root");
    std::fs::write(
        root.path().join("maenifold.json"),
        r#"{
            "mcpServers": [
                {
                    "type": "stdio",
                    "name": "workiq",
                    "command": "/usr/bin/env",
                    "args": ["agency", "mcp", "workiq"],
                    "gateway": true
                }
            ],
            "agents": {}
        }"#,
    )
    .expect("write legacy config");

    let error = AgentsConfig::load(root.path()).expect_err("legacy array form must be rejected");
    let message = error.to_string();
    assert!(
        message.contains("mcpServers")
            && message.contains("object")
            && message.contains("migration"),
        "error should clearly point to the object-keyed migration: {message}"
    );
}

#[test]
fn mcp_servers_schema_load_accepts_mixed_object_keyed_config() {
    let root = tempfile::tempdir().expect("create temp root");
    std::fs::write(
        root.path().join("maenifold.json"),
        r#"{
            "mcpServers": {
                "workiq": {
                    "command": "/usr/bin/env",
                    "args": ["agency", "mcp", "workiq"],
                    "gateway": true
                },
                "learn": {
                    "url": "https://learn.microsoft.com/api/mcp",
                    "gateway": true
                }
            },
            "agents": {}
        }"#,
    )
    .expect("write object-keyed config");

    let config = AgentsConfig::load(root.path()).expect("object-keyed mcpServers should load");
    assert_mcp_server_map(&config.mcp_servers);
    assert!(config.mcp_servers.contains_key("workiq"));
    assert!(config.mcp_servers.contains_key("learn"));
    assert!(matches!(
        config.mcp_servers.get("workiq").expect("workiq"),
        McpServerEntry::Stdio { command, gateway, .. }
            if command == "/usr/bin/env" && *gateway
    ));
    assert!(matches!(
        config.mcp_servers.get("learn").expect("learn"),
        McpServerEntry::Http { url, gateway, .. }
            if url == "https://learn.microsoft.com/api/mcp" && *gateway
    ));
}

#[test]
fn mcp_servers_schema_load_preserves_all_live_server_entries() {
    let root = tempfile::tempdir().expect("create temp root");
    std::fs::write(
        root.path().join("maenifold.json"),
        live_config_object_keyed_fixture(),
    )
    .expect("write migrated live config fixture");

    let config = AgentsConfig::load(root.path()).expect("migrated live config should load");
    assert_mcp_server_map(&config.mcp_servers);
    assert_eq!(config.mcp_servers.len(), 10);
    assert_http(
        &config.mcp_servers,
        "maenifold",
        "http://localhost:8080/mcp",
        false,
    );
    assert_http(
        &config.mcp_servers,
        "microsoft-learn",
        "https://learn.microsoft.com/api/mcp",
        false,
    );
    assert_stdio(
        &config.mcp_servers,
        "workiq",
        "/opt/homebrew/bin/node",
        &["/Users/brett/.npm-global/bin/workiq", "mcp"],
        true,
    );
    assert_stdio(
        &config.mcp_servers,
        "fetch",
        "/Users/brett/.local/bin/uvx",
        &["mcp-server-fetch"],
        true,
    );
    assert_stdio(
        &config.mcp_servers,
        "srectl",
        "/Users/brett/.dotnet/tools/srectl",
        &["mcp", "start"],
        true,
    );
    assert_stdio(
        &config.mcp_servers,
        "azure-mcp-server",
        "/opt/homebrew/bin/npx",
        &["-y", "@azure/mcp@latest", "server", "start"],
        true,
    );
    assert_eq!(
        stdio_env(&config.mcp_servers, "azure-mcp-server")
            .get("PATH")
            .map(String::as_str),
        Some("/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin")
    );
    assert_stdio(
        &config.mcp_servers,
        "discovery-workbench",
        "/Users/brett/.local/bin/uv",
        &[
            "run",
            "--with",
            "mcp>=1.0.0",
            "--with",
            "pydantic>=2.0.0",
            "--with",
            "requests>=2.25.0",
            "--with",
            "azure-identity>=1.12.0",
            "--with",
            "azure-storage-blob>=12.0.0",
            "--with",
            "docker>=6.0.0",
            "--with",
            "pyyaml>=6.0.0",
            "--with",
            "jsonschema>=4.0.0",
            "python",
            "/Users/brett/src/discovery/utils/agent-workbench/mcp-server/server.py",
        ],
        true,
    );
    assert_stdio(
        &config.mcp_servers,
        "ai-infrastructure",
        "/Users/brett/.local/bin/uv",
        &[
            "run",
            "--with",
            "fastmcp==2.14.0",
            "--with",
            "paramiko>=3.4.0",
            "--directory",
            "/Users/brett/src/discovery/6-solutions/common-infra/CC-MCP/ai-infrastructure-mcp-server",
            "python",
            "-m",
            "ai_infrastructure_mcp.server",
            "--mode",
            "stdio",
        ],
        true,
    );
    assert_stdio(
        &config.mcp_servers,
        "mcp-apps-demo",
        "/opt/homebrew/bin/node",
        &[
            "/tmp/ext-apps/examples/basic-server-vanillajs/dist/index.js",
            "--stdio",
        ],
        true,
    );
    assert_http(
        &config.mcp_servers,
        "elicitation-form",
        "http://localhost:3000/mcp",
        true,
    );
}

fn assert_mcp_server_map(_servers: &HashMap<String, McpServerEntry>) {}

fn assert_stdio(
    servers: &HashMap<String, McpServerEntry>,
    name: &str,
    command: &str,
    args: &[&str],
    expected_gateway: bool,
) {
    match servers.get(name).expect("server should exist") {
        McpServerEntry::Stdio {
            command: actual_command,
            args: actual_args,
            gateway,
            ..
        } => {
            assert_eq!(actual_command, command);
            assert_eq!(
                actual_args,
                &args
                    .iter()
                    .map(|value| value.to_string())
                    .collect::<Vec<_>>()
            );
            assert_eq!(*gateway, expected_gateway);
        }
        other => panic!("{name} should be stdio, got {other:?}"),
    }
}

fn assert_http(
    servers: &HashMap<String, McpServerEntry>,
    name: &str,
    url: &str,
    expected_gateway: bool,
) {
    match servers.get(name).expect("server should exist") {
        McpServerEntry::Http {
            url: actual_url,
            gateway,
            ..
        } => {
            assert_eq!(actual_url, url);
            assert_eq!(*gateway, expected_gateway);
        }
        other => panic!("{name} should be http, got {other:?}"),
    }
}

fn stdio_env<'a>(
    servers: &'a HashMap<String, McpServerEntry>,
    name: &str,
) -> &'a HashMap<String, String> {
    match servers.get(name).expect("server should exist") {
        McpServerEntry::Stdio { env, .. } => env,
        other => panic!("{name} should be stdio, got {other:?}"),
    }
}

fn live_config_object_keyed_fixture() -> &'static str {
    r#"{
        "mcpServers": {
            "maenifold": { "url": "http://localhost:8080/mcp" },
            "microsoft-learn": { "url": "https://learn.microsoft.com/api/mcp" },
            "workiq": {
                "command": "/opt/homebrew/bin/node",
                "args": ["/Users/brett/.npm-global/bin/workiq", "mcp"],
                "gateway": true
            },
            "fetch": {
                "command": "/Users/brett/.local/bin/uvx",
                "args": ["mcp-server-fetch"],
                "gateway": true
            },
            "srectl": {
                "command": "/Users/brett/.dotnet/tools/srectl",
                "args": ["mcp", "start"],
                "gateway": true
            },
            "azure-mcp-server": {
                "command": "/opt/homebrew/bin/npx",
                "args": ["-y", "@azure/mcp@latest", "server", "start"],
                "env": {
                    "PATH": "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"
                },
                "gateway": true
            },
            "discovery-workbench": {
                "command": "/Users/brett/.local/bin/uv",
                "args": [
                    "run", "--with", "mcp>=1.0.0", "--with", "pydantic>=2.0.0",
                    "--with", "requests>=2.25.0", "--with", "azure-identity>=1.12.0",
                    "--with", "azure-storage-blob>=12.0.0", "--with", "docker>=6.0.0",
                    "--with", "pyyaml>=6.0.0", "--with", "jsonschema>=4.0.0",
                    "python", "/Users/brett/src/discovery/utils/agent-workbench/mcp-server/server.py"
                ],
                "gateway": true
            },
            "ai-infrastructure": {
                "command": "/Users/brett/.local/bin/uv",
                "args": [
                    "run", "--with", "fastmcp==2.14.0", "--with", "paramiko>=3.4.0",
                    "--directory",
                    "/Users/brett/src/discovery/6-solutions/common-infra/CC-MCP/ai-infrastructure-mcp-server",
                    "python", "-m", "ai_infrastructure_mcp.server", "--mode", "stdio"
                ],
                "gateway": true
            },
            "mcp-apps-demo": {
                "command": "/opt/homebrew/bin/node",
                "args": ["/tmp/ext-apps/examples/basic-server-vanillajs/dist/index.js", "--stdio"],
                "gateway": true
            },
            "elicitation-form": {
                "url": "http://localhost:3000/mcp",
                "gateway": true
            }
        },
        "agents": {}
    }"#
}
