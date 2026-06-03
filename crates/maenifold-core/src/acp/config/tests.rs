// T-01.16: settings deserialization tests extracted from acp/config.rs to keep
// each module ≤300 LOC. No behavioural changes.

use super::*;
use serde_json::json;

fn sample_settings_json() -> serde_json::Value {
    json!({
        "logRotation": "hourly",
        "debounceMs": 250,
        "autoSync": false,
        "syncLogging": false,
        "sessionCleanup": false,
        "sqliteBusyTimeoutMs": 7000,
        "acpConnectionTimeoutSeconds": 90,
        "syncLeaseMs": 15000,
        "watcherBufferSize": 131072,
        "incrementalOptimizeEvery": 55,
        "incrementalVacuumMinutes": 360,
        "sessionAbandonMinutes": 45,
        "snippetLength": 512,
        "syncRole": "leader",
        "decay": {
            "graceDaysSequential": 9,
            "graceDaysWorkflows": 18,
            "graceDaysDefault": 35,
            "halfLifeDays": 45,
            "function": "exponential"
        },
        "community": {
            "louvainGamma": 1.25,
            "minSharedNeighbors": 5,
            "minOverlap": 0.55,
            "maxSiblings": 12,
            "debounceMs": 3500,
            "nmiThreshold": 0.9,
            "recomputeFileThreshold": 100
        },
        "vector": {
            "embeddingLogs": true,
            "vectorLogs": true,
            "disableVector": true
        },
        "debug": {
            "trace": true,
            "pipelineDriver": "inline"
        }
    })
}

fn expected_settings() -> MaenifoldSettings {
    MaenifoldSettings {
        log_rotation: "hourly".to_string(),
        debounce_ms: 250,
        auto_sync: false,
        sync_logging: false,
        session_cleanup: false,
        sqlite_busy_timeout_ms: 7000,
        acp_connection_timeout_seconds: 90,
        sync_lease_ms: 15000,
        watcher_buffer_size: 131072,
        incremental_optimize_every: 55,
        incremental_vacuum_minutes: 360,
        session_abandon_minutes: Some(45),
        quarantine_retention_days: None,
        snippet_length: Some(512),
        sync_role: Some("leader".to_string()),
        default_agent: None,
        decay: DecaySettings {
            grace_days_sequential: 9,
            grace_days_workflows: 18,
            grace_days_default: 35,
            half_life_days: 45,
            function: "exponential".to_string(),
        },
        community: CommunitySettings {
            louvain_gamma: 1.25,
            min_shared_neighbors: 5,
            min_overlap: 0.55,
            max_siblings: 12,
            debounce_ms: 3500,
            nmi_threshold: 0.9,
            recompute_file_threshold: Some(100),
        },
        vector: VectorSettings {
            embedding_logs: true,
            vector_logs: true,
            disable_vector: true,
            active_model: None,
        },
        debug: DebugSettings {
            trace: true,
            pipeline_driver: Some("inline".to_string()),
        },
        gateway: GatewaySettings::default(),
        env_passthrough: Vec::new(),
    }
}

#[test]
fn settings_deserializes_from_json() {
    let settings: MaenifoldSettings =
        serde_json::from_value(sample_settings_json()).expect("settings should deserialize");

    assert_eq!(settings, expected_settings());
}

#[test]
fn settings_defaults_when_absent() {
    let config: AgentsConfig = serde_json::from_value(json!({
        "agents": {},
        "mcpServers": {},
        "settings": {}
    }))
    .expect("config should deserialize");

    let settings = config.settings.expect("settings should be present");

    assert_eq!(settings.log_rotation, "daily");
    assert_eq!(settings.debounce_ms, 150);
    assert!(settings.auto_sync);
    assert!(settings.sync_logging);
    assert!(settings.session_cleanup);
    assert_eq!(settings.sqlite_busy_timeout_ms, 5000);
    assert_eq!(settings.acp_connection_timeout_seconds, 60);
    assert_eq!(settings.sync_lease_ms, 10000);
    assert_eq!(settings.watcher_buffer_size, 65536);
    assert_eq!(settings.incremental_optimize_every, 40);
    assert_eq!(settings.incremental_vacuum_minutes, 720);
    assert_eq!(settings.decay.grace_days_sequential, 7);
    assert_eq!(settings.decay.grace_days_workflows, 14);
    assert_eq!(settings.decay.grace_days_default, 28);
    assert_eq!(settings.decay.half_life_days, 30);
    assert_eq!(settings.decay.function, "power-law");
    assert_eq!(settings.community.louvain_gamma, 1.0);
    assert_eq!(settings.community.min_shared_neighbors, 3);
    assert_eq!(settings.community.min_overlap, 0.4);
    assert_eq!(settings.community.max_siblings, 10);
    assert_eq!(settings.community.debounce_ms, 2000);
    assert_eq!(settings.community.nmi_threshold, 0.8);
    assert_eq!(settings.community.recompute_file_threshold, None);
    assert!(!settings.vector.embedding_logs);
    assert!(!settings.vector.vector_logs);
    assert!(!settings.vector.disable_vector);
    assert!(!settings.debug.trace);
    assert_eq!(settings.debug.pipeline_driver, None);
    assert_eq!(settings.sync_role, None);
    assert_eq!(settings.session_abandon_minutes, None);
    assert_eq!(settings.snippet_length, None);
}

#[test]
fn settings_roundtrips_through_json() {
    let settings = MaenifoldSettings::default();
    let json = serde_json::to_string(&settings).expect("settings should serialize");
    let reparsed: MaenifoldSettings =
        serde_json::from_str(&json).expect("settings should deserialize");

    assert_eq!(reparsed, settings);
}

#[test]
fn agents_config_includes_settings() {
    let config: AgentsConfig = serde_json::from_value(json!({
        "mcpServers": {
            "local": {
                "command": "/bin/echo",
                "args": ["hello"]
            },
            "remote": {
                "url": "http://localhost:3000/mcp"
            }
        },
        "agents": {
            "planner": {
                "command": "/bin/echo --plan",
                "env": {
                    "MODE": "test"
                },
                "mcpServers": {
                    "local": {
                        "command": "/bin/echo",
                        "args": ["planner"]
                    }
                }
            }
        },
        "settings": sample_settings_json()
    }))
    .expect("config should deserialize");

    let settings = config.settings.expect("settings should be present");
    assert_eq!(settings, expected_settings());
    assert_eq!(config.mcp_servers.len(), 2);
    assert!(config.mcp_servers.contains_key("local"));
    assert!(config.mcp_servers.contains_key("remote"));

    let agent = config
        .agents
        .get("planner")
        .expect("planner agent should exist");
    assert_eq!(agent.command, "/bin/echo --plan");
    assert_eq!(agent.env.get("MODE").map(String::as_str), Some("test"));
    assert_eq!(agent.mcp_servers.len(), 1);
    assert!(agent.mcp_servers.contains_key("local"));
}

fn first_stdio(entry: serde_json::Value) -> McpServerEntry {
    serde_json::from_value::<AgentsConfig>(json!({"mcpServers": {"local": entry}, "agents": {}}))
        .expect("config")
        .mcp_servers
        .into_iter()
        .map(|(_name, entry)| entry)
        .next()
        .expect("stdio entry")
}
fn first_agent(entry: serde_json::Value) -> AgentEntry {
    serde_json::from_value::<AgentsConfig>(json!({"agents": {"agent": entry}}))
        .expect("config")
        .agents
        .into_values()
        .next()
        .expect("agent entry")
}

#[test]
fn stdio_entry_deserializes_env_from_file_only() {
    let entry = first_stdio(
        json!({"command":"/bin/echo","envFromFile":{"AZURE_OPENAI_API_KEY":"~/.secrets/azure"}}),
    );
    let McpServerEntry::Stdio {
        env,
        env_from_file,
        env_from_command,
        ..
    } = entry
    else {
        panic!("stdio")
    };
    assert!(env.is_empty() && env_from_command.is_empty());
    assert_eq!(
        env_from_file
            .get("AZURE_OPENAI_API_KEY")
            .map(String::as_str),
        Some("~/.secrets/azure")
    );
}

#[test]
fn agent_entry_deserializes_env_from_command_only() {
    let entry = first_agent(
        json!({"command":"/bin/echo --agent","envFromCommand":{"ANTHROPIC_API_KEY":{"command":"/usr/bin/security","args":["find-generic-password","-w"]}}}),
    );
    assert!(entry.env.is_empty() && entry.env_from_file.is_empty());
    let command = entry
        .env_from_command
        .get("ANTHROPIC_API_KEY")
        .expect("command source");
    assert_eq!(
        (&command.command, &command.args),
        (
            &"/usr/bin/security".to_string(),
            &vec!["find-generic-password".to_string(), "-w".to_string()]
        )
    );
}

#[test]
fn stdio_entry_deserializes_all_env_sources() {
    let entry = first_stdio(
        json!({"command":"/bin/echo","env":{"PATH":"/usr/bin"},"envFromFile":{"OPENAI_API_KEY":"~/.secrets/openai"},"envFromCommand":{"ANTHROPIC_API_KEY":{"command":"/usr/bin/security","args":["find-generic-password"]}}}),
    );
    let McpServerEntry::Stdio {
        env,
        env_from_file,
        env_from_command,
        ..
    } = entry
    else {
        panic!("stdio")
    };
    assert_eq!(env.get("PATH").map(String::as_str), Some("/usr/bin"));
    assert_eq!(
        env_from_file.get("OPENAI_API_KEY").map(String::as_str),
        Some("~/.secrets/openai")
    );
    assert_eq!(
        env_from_command
            .get("ANTHROPIC_API_KEY")
            .expect("command source")
            .args,
        vec!["find-generic-password"]
    );
}

#[test]
fn agent_entry_env_sources_default_empty() {
    let entry = first_agent(json!({"command":"/bin/echo --agent"}));
    assert!(
        entry.env.is_empty() && entry.env_from_file.is_empty() && entry.env_from_command.is_empty()
    );
}

// Removed: test_real_maenifold_json_core was a dev-machine smoke test (read
// developer's actual $HOME/maenifold/maenifold.json). Deleted in favor of
// proper unit tests that own their fixtures.

#[test]
fn agents_config_settings_optional() {
    let config: AgentsConfig = serde_json::from_value(json!({
        "agents": {
            "planner": {
                "command": "/bin/echo --plan"
            }
        }
    }))
    .expect("config should deserialize");

    assert_eq!(config.settings, None);
    let agent = config
        .agents
        .get("planner")
        .expect("planner agent should exist");
    assert_eq!(agent.command, "/bin/echo --plan");
    assert!(agent.env.is_empty());
    assert!(agent.mcp_servers.is_empty());
}

#[test]
fn agents_config_load_errors_when_config_missing() {
    let root = tempfile::tempdir().expect("create temp root");
    let error = AgentsConfig::load(root.path()).expect_err("missing config should error");

    assert!(
        error.to_string().contains("not found"),
        "unexpected error: {error}"
    );
}

#[test]
fn agents_config_load_errors_when_config_unparsable() {
    let root = tempfile::tempdir().expect("create temp root");
    std::fs::write(root.path().join("maenifold.json"), "{ invalid json")
        .expect("write invalid config");

    let error = AgentsConfig::load(root.path()).expect_err("invalid config should error");

    assert!(
        error.to_string().contains("failed to parse agents config"),
        "unexpected error: {error}"
    );
}
