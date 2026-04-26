use super::{loader::validate_intent_resolver_config, *};
use serde_json::json;
use std::collections::HashMap;

fn config_with_agent(name: &str) -> AgentsConfig {
    let mut agents = HashMap::new();
    agents.insert(
        name.to_string(),
        AgentEntry { command: "/bin/echo".to_string(), env: HashMap::new(), mcp_servers: Vec::new() },
    );
    AgentsConfig { mcp_servers: Vec::new(), agents, settings: None }
}

fn validation_error(settings: IntentResolverSettings) -> String {
    validate_intent_resolver_config(&settings, &config_with_agent("resolver"))
        .expect_err("settings should fail validation")
        .to_string()
}

#[test]
fn gateway_defaults_when_absent() {
    let cfg: AgentsConfig = serde_json::from_value(json!({"agents": {}, "settings": {}}))
        .expect("config should deserialize");
    let resolver = cfg.settings.expect("settings").gateway.intent_resolver;
    assert!(!resolver.enabled);
    assert_eq!(resolver.model, "gpt-5.4-mini");
    assert_eq!(resolver.auto_execute_confidence_threshold, 0.9);
    assert_eq!(resolver.max_catalog_characters, 24000);
    assert!(resolver.mcp_servers.is_empty());
}

#[test]
fn explicit_gateway_settings_deserialize() {
    let settings: GatewaySettings = serde_json::from_value(json!({
        "intentResolver": {
            "enabled": true, "agent": "resolver", "model": "gpt-5.4",
            "thoughtLevel": "high", "timeoutSeconds": 30, "maxCandidates": 8,
            "autoExecuteReadSafe": true, "autoExecuteConfidenceThreshold": 0.75,
            "maxCatalogCharacters": 12000,
            "mcpServers": [{"type": "stdio", "name": "tools", "command": "/bin/echo", "args": ["ok"]}]
        }
    })).expect("gateway settings should deserialize");

    let resolver = settings.intent_resolver;
    assert!(resolver.enabled);
    assert_eq!(resolver.agent.as_deref(), Some("resolver"));
    assert_eq!(resolver.model, "gpt-5.4");
    assert_eq!(resolver.effective_thought_level(), Some("high"));
    assert_eq!(resolver.timeout_seconds, Some(30));
    assert_eq!(resolver.max_candidates, Some(8));
    assert!(resolver.auto_execute_read_safe);
    assert_eq!(resolver.auto_execute_confidence_threshold, 0.75);
    assert_eq!(resolver.max_catalog_characters, 12000);
    assert_eq!(resolver.mcp_servers.len(), 1);
}

#[test]
fn valid_enabled_resolver_passes_validation() {
    let s = IntentResolverSettings {
        enabled: true, agent: Some("resolver".to_string()),
        thought_level: Some("medium".to_string()), ..IntentResolverSettings::default()
    };
    validate_intent_resolver_config(&s, &config_with_agent("resolver")).expect("valid resolver should pass");
}

#[test]
fn enabled_resolver_requires_known_non_empty_agent() {
    for agent in [None, Some("".to_string()), Some("   ".to_string())] {
        let settings = IntentResolverSettings {
            enabled: true,
            agent,
            ..IntentResolverSettings::default()
        };
        assert!(validation_error(settings).contains("requires a non-empty agent"));
    }

    let settings = IntentResolverSettings {
        enabled: true, agent: Some("missing".to_string()),
        ..IntentResolverSettings::default()
    };
    assert!(validation_error(settings).contains("not configured or is not executable"));
}

#[test]
fn rejects_invalid_numeric_and_thought_level_settings() {
    let mut s = IntentResolverSettings { auto_execute_confidence_threshold: 1.1, ..IntentResolverSettings::default() };
    assert!(validation_error(s.clone()).contains("between 0.0 and 1.0"));

    s.auto_execute_confidence_threshold = 0.9;
    s.thought_level = Some("none".to_string());
    assert_eq!(s.effective_thought_level(), None);
    validate_intent_resolver_config(&s, &AgentsConfig::default()).expect("thoughtLevel none should validate");

    s.thought_level = Some("ultra".to_string());
    assert!(validation_error(s.clone()).contains("unsupported"));

    s.thought_level = None;
    s.timeout_seconds = Some(0);
    assert!(validation_error(s.clone()).contains("timeoutSeconds"));

    s.timeout_seconds = None;
    s.max_candidates = Some(0);
    assert!(validation_error(s).contains("maxCandidates"));
}

#[test]
fn existing_configs_without_gateway_section_parse() {
    let cfg: AgentsConfig = serde_json::from_value(json!({
        "mcpServers": [], "agents": {}, "settings": {"logRotation": "hourly"}
    })).expect("legacy config should deserialize");
    let settings = cfg.settings.expect("settings should be present");
    assert_eq!(settings.log_rotation, "hourly");
    assert!(!settings.gateway.intent_resolver.enabled);
}
