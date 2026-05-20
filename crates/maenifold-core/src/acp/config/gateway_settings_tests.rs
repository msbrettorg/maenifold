use super::{loader::validate_intent_resolver_config, *};
use serde_json::json;
use std::collections::HashMap;

fn config_with_agent(name: &str) -> AgentsConfig {
    let mut agents = HashMap::new();
    agents.insert(
        name.to_string(),
        AgentEntry {
            command: "/bin/echo".to_string(),
            env: HashMap::new(),
            env_from_file: HashMap::new(),
            env_from_command: HashMap::new(),
            mcp_servers: HashMap::new(),
        },
    );
    AgentsConfig {
        mcp_servers: HashMap::new(),
        agents,
        settings: None,
    }
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
            "enabled": true, "model": "gpt-5.4",
            "thoughtLevel": "high", "timeoutSeconds": 30, "maxCandidates": 8,
            "autoExecuteReadSafe": true, "autoExecuteConfidenceThreshold": 0.75,
            "maxCatalogCharacters": 12000,
            "mcpServers": {"tools": {"command": "/bin/echo", "args": ["ok"]}}
        }
    }))
    .expect("gateway settings should deserialize");

    let resolver = settings.intent_resolver;
    assert!(resolver.enabled);
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
        enabled: true,
        thought_level: Some("medium".to_string()),
        ..IntentResolverSettings::default()
    };
    validate_intent_resolver_config(&s, &config_with_agent("resolver"))
        .expect("valid resolver should pass");
}

#[test]
fn enabled_resolver_does_not_require_an_agent() {
    // Sanity: enabled resolver with NO ACP agent config still validates.
    // GatewayIntent resolver paths are direct inferencing only.
    let settings = IntentResolverSettings {
        enabled: true,
        ..IntentResolverSettings::default()
    };
    validate_intent_resolver_config(&settings, &AgentsConfig::default())
        .expect("enabled resolver should not require an agent");
}

#[test]
fn rejects_invalid_numeric_and_thought_level_settings() {
    let mut s = IntentResolverSettings {
        auto_execute_confidence_threshold: 1.1,
        ..IntentResolverSettings::default()
    };
    assert!(validation_error(s.clone()).contains("between 0.0 and 1.0"));

    s.auto_execute_confidence_threshold = 0.9;
    s.thought_level = Some("none".to_string());
    assert_eq!(s.effective_thought_level(), None);
    validate_intent_resolver_config(&s, &AgentsConfig::default())
        .expect("thoughtLevel none should validate");

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
fn threshold_boundaries_validate() {
    let mut s = IntentResolverSettings {
        auto_execute_confidence_threshold: -0.1,
        ..IntentResolverSettings::default()
    };
    assert!(validation_error(s.clone()).contains("between 0.0 and 1.0"));
    s.auto_execute_confidence_threshold = 0.0;
    validate_intent_resolver_config(&s, &AgentsConfig::default())
        .expect("0.0 threshold should validate");
    s.auto_execute_confidence_threshold = 1.0;
    validate_intent_resolver_config(&s, &AgentsConfig::default())
        .expect("1.0 threshold should validate");
}

#[test]
fn gateway_settings_roundtrip_uses_camel_case() {
    let settings = GatewaySettings {
        intent_resolver: IntentResolverSettings {
            enabled: true,
            model: "gpt-5.4".to_string(),
            thought_level: Some("low".to_string()),
            timeout_seconds: Some(5),
            max_candidates: Some(3),
            auto_execute_read_safe: true,
            auto_execute_confidence_threshold: 0.5,
            max_catalog_characters: 1234,
            mcp_servers: HashMap::from([(
                "m".to_string(),
                McpServerEntry::Http {
                    url: "http://localhost".to_string(),
                    gateway: false,
                },
            )]),
            direct_inference: None,
        },
    };
    let value = serde_json::to_value(&settings).expect("gateway settings should serialize");
    let resolver = &value["intentResolver"];
    for key in [
        "thoughtLevel",
        "timeoutSeconds",
        "maxCandidates",
        "autoExecuteReadSafe",
        "autoExecuteConfidenceThreshold",
        "maxCatalogCharacters",
        "mcpServers",
    ] {
        assert!(resolver.get(key).is_some(), "missing {key}");
    }
    assert!(value.get("intentResolver").is_some());
    let reparsed: GatewaySettings =
        serde_json::from_value(value).expect("gateway settings should deserialize");
    assert_eq!(reparsed, settings);
}

#[test]
fn existing_configs_without_gateway_section_parse() {
    let cfg: AgentsConfig = serde_json::from_value(json!({
        "mcpServers": {}, "agents": {}, "settings": {"logRotation": "hourly"}
    }))
    .expect("legacy config should deserialize");
    let settings = cfg.settings.expect("settings should be present");
    assert_eq!(settings.log_rotation, "hourly");
    assert!(!settings.gateway.intent_resolver.enabled);
}
