use super::*;

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

    assert!(config.settings.expect("settings").gateway.intent_resolver.enabled);
}
