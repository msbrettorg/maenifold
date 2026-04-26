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
fn agents_config_load_or_default_propagates_intent_resolver_validation_errors() {
    let root = tempfile::tempdir().expect("create temp root");
    std::fs::write(
        root.path().join("maenifold.json"),
        r#"{
            "agents": {},
            "settings": {
                "gateway": {
                    "intentResolver": {
                        "enabled": true,
                        "agent": "missing-resolver"
                    }
                }
            }
        }"#,
    )
    .expect("write invalid resolver config");

    let error = AgentsConfig::load_or_default(root.path())
        .expect_err("invalid resolver config should surface");

    assert!(
        error
            .to_string()
            .contains("gateway intent resolver agent 'missing-resolver'"),
        "unexpected error: {error}"
    );
}
