use super::config_tools::{partial_settings_schema, tool};
use crate::acp_config::{AgentEntry, HttpAuthConfig, McpServerEntry};
use crate::mcp::{domain_mapping, gateway_config};
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap};

const CONFIG_GATEWAY_COMMANDS: [&str; 8] = [
    "GetSettingsSchema",
    "SetSettings",
    "AddAgent",
    "UpdateAgent",
    "DeleteAgent",
    "AddMcpServer",
    "UpdateMcpServer",
    "DeleteMcpServer",
];

fn required_set(schema: &Value) -> BTreeSet<String> {
    schema
        .get("required")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn object_properties(schema: &Value) -> &serde_json::Map<String, Value> {
    schema
        .get("properties")
        .and_then(Value::as_object)
        .expect("schema must have properties map")
}

fn tool_property_schema(tool_name: &str, property_name: &str) -> Value {
    tool(tool_name)
        .unwrap_or_else(|| panic!("{tool_name} schema should exist"))
        .input_schema
        .get("properties")
        .and_then(Value::as_object)
        .and_then(|properties| properties.get(property_name))
        .cloned()
        .unwrap_or_else(|| panic!("{tool_name} should expose `{property_name}`"))
}

#[test]
fn partial_settings_schema_exposes_core_top_level_keys() {
    let schema = partial_settings_schema();
    let properties = object_properties(&schema);

    for key in ["logRotation", "debounceMs", "autoSync"] {
        assert!(
            properties.contains_key(key),
            "partial settings schema should expose `{key}`"
        );
    }
}

#[test]
fn add_agent_schema_accepts_optional_args() {
    let schema = tool_property_schema("AddAgent", "config");
    let properties = object_properties(&schema);

    assert!(
        properties.contains_key("args"),
        "agent entry schema should expose an optional `args` array"
    );
    assert!(
        !required_set(&schema).contains("args"),
        "agent entry schema must not require `args`"
    );

    let entry = AgentEntry {
        command: r"C:\Program Files\nodejs\npx.cmd".to_string(),
        args: Some(vec![
            "-y".to_string(),
            "@modelcontextprotocol/server-filesystem".to_string(),
        ]),
        env: HashMap::new(),
        env_from_file: HashMap::new(),
        env_from_command: HashMap::new(),
        mcp_servers: HashMap::new(),
    };
    let instance = serde_json::to_value(entry).expect("agent entry should serialize");
    let validator = jsonschema::validator_for(&schema).expect("agent schema should compile");

    validator.validate(&instance).unwrap_or_else(|error| {
        panic!("agent schema should accept serialized args fixture: {error}; instance={instance}")
    });
}

#[test]
fn add_mcp_server_schema_accepts_http_headers_auth_and_timeout() {
    let schema = tool_property_schema("AddMcpServer", "entry");
    let http_variant = schema
        .get("oneOf")
        .and_then(Value::as_array)
        .and_then(|variants| {
            variants
                .iter()
                .find(|variant| required_set(variant).contains("url"))
        })
        .expect("HTTP mcp server schema variant should exist");
    let properties = object_properties(http_variant);

    for key in ["headers", "auth", "timeout"] {
        assert!(
            properties.contains_key(key),
            "HTTP mcp server schema should expose optional `{key}`"
        );
        assert!(
            !required_set(http_variant).contains(key),
            "HTTP mcp server schema must not require `{key}`"
        );
    }

    let entry = McpServerEntry::Http {
        url: "https://example.invalid/mcp".to_string(),
        headers: HashMap::from([("X-Test-Header".to_string(), "marker".to_string())]),
        auth: Some(HttpAuthConfig::AzureDefault),
        timeout: Some(30),
        gateway: false,
    };
    let instance = serde_json::to_value(entry).expect("http mcp entry should serialize");
    let validator = jsonschema::validator_for(&schema).expect("mcp server schema should compile");

    validator.validate(&instance).unwrap_or_else(|error| {
        panic!("mcp server schema should accept HTTP auth fixture: {error}; instance={instance}")
    });
}

#[test]
fn add_agent_schema_rejects_bogus_extra_fields() {
    let schema = tool_property_schema("AddAgent", "config");
    let invalid = json!({
        "command": "x",
        "bogus": true
    });
    let validator = jsonschema::validator_for(&schema).expect("agent schema should compile");

    assert_eq!(
        schema.get("additionalProperties"),
        Some(&Value::Bool(false)),
        "agent entry schema must set additionalProperties=false"
    );
    assert!(
        validator.validate(&invalid).is_err(),
        "agent entry schema should reject unknown fields: {invalid}"
    );
}

#[test]
fn config_gateway_commands_register_under_maenifold_system() {
    let expected = CONFIG_GATEWAY_COMMANDS
        .into_iter()
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    let actual = gateway_config::commands()
        .into_iter()
        .map(|entry| entry.name)
        .collect::<BTreeSet<_>>();

    assert_eq!(actual, expected, "gateway config command set should match");

    for name in CONFIG_GATEWAY_COMMANDS {
        assert_eq!(
            domain_mapping::tool_domain(name),
            Some("maenifold.system"),
            "{name} should resolve to maenifold.system"
        );
    }
}
