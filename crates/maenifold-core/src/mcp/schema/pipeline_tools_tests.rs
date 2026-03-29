use super::pipeline_tools::tool;
use serde_json::Value;

#[test]
#[allow(clippy::too_many_lines)]
fn pipeline_schema_includes_all_new_operation_fields_with_expected_types() {
    let tool = tool("Pipeline").expect("pipeline tool schema");
    let properties = tool
        .input_schema
        .get("properties")
        .and_then(Value::as_object)
        .expect("pipeline properties");

    for field in ["run", "pause", "resume", "cancel", "delete", "list"] {
        let property = properties
            .get(field)
            .and_then(Value::as_object)
            .expect("boolean property");
        assert_eq!(
            property.get("type").and_then(Value::as_str),
            Some("boolean")
        );
        assert!(property
            .get("description")
            .and_then(Value::as_str)
            .is_some_and(|description| !description.is_empty()));
    }

    let rewind = properties
        .get("rewind")
        .and_then(Value::as_object)
        .expect("rewind property");
    assert_eq!(rewind.get("type").and_then(Value::as_str), Some("object"));
    assert_eq!(
        rewind
            .get("required")
            .and_then(Value::as_array)
            .and_then(|required| required.first())
            .and_then(Value::as_str),
        Some("nodeIds")
    );
    assert_eq!(
        rewind
            .get("properties")
            .and_then(Value::as_object)
            .and_then(|properties| properties.get("nodeIds"))
            .and_then(Value::as_object)
            .and_then(|node_ids| node_ids.get("type"))
            .and_then(Value::as_str),
        Some("array")
    );

    let graft = properties
        .get("graft")
        .and_then(Value::as_object)
        .expect("graft property");
    assert_eq!(graft.get("type").and_then(Value::as_str), Some("object"));
    let graft_properties = graft
        .get("properties")
        .and_then(Value::as_object)
        .expect("graft properties");
    assert_eq!(
        graft_properties
            .get("nodes")
            .and_then(Value::as_object)
            .and_then(|nodes| nodes.get("type"))
            .and_then(Value::as_str),
        Some("array")
    );
    assert_eq!(
        graft_properties
            .get("edges")
            .and_then(Value::as_object)
            .and_then(|edges| edges.get("type"))
            .and_then(Value::as_str),
        Some("array")
    );

    let join = properties
        .get("join")
        .and_then(Value::as_object)
        .expect("join property");
    assert_eq!(join.get("type").and_then(Value::as_str), Some("object"));
    assert_eq!(
        join.get("required")
            .and_then(Value::as_array)
            .map(|values| values.iter().filter_map(Value::as_str).collect::<Vec<_>>()),
        Some(vec!["sourceSessionId", "edges"])
    );

    let fork = properties
        .get("fork")
        .and_then(Value::as_object)
        .expect("fork property");
    assert_eq!(fork.get("type").and_then(Value::as_str), Some("object"));
    assert_eq!(
        fork.get("required")
            .and_then(Value::as_array)
            .map(|values| values.iter().filter_map(Value::as_str).collect::<Vec<_>>()),
        Some(vec!["fromNodeId"])
    );
    assert_eq!(
        fork.get("properties")
            .and_then(Value::as_object)
            .and_then(|properties| properties.get("newSessionId"))
            .and_then(Value::as_object)
            .and_then(|property| property.get("type"))
            .and_then(Value::as_str),
        Some("string")
    );
}

#[test]
fn pipeline_schema_definition_nodes_advertise_supported_config_fields() {
    let definition = definition_node_schema();
    let variants = node_variants(&definition);

    assert_variant_supports_fields(&variants, "prompt", &["model", "thoughtLevel", "allowedTools", "maxTurns", "timeout", "toolMode", "agentConfig"]);
    assert_variant_supports_fields(&variants, "sequentialThinking", &["model", "thoughtLevel", "allowedTools", "maxTurns", "timeout", "agentConfig"]);
    assert_variant_supports_fields(&variants, "tool", &["model", "thoughtLevel", "allowedTools", "maxTurns", "agentConfig"]);
    assert_variant_required_fields(&variants);
    assert_agent_config_fields(&variants, "prompt");
    assert_agent_config_fields(&variants, "sequentialThinking");
    assert_agent_config_fields(&variants, "tool");
}
#[test]
fn pipeline_schema_graft_nodes_advertise_supported_config_fields() {
    let graft = graft_node_schema();
    let variants = node_variants(&graft);

    assert_variant_supports_fields(&variants, "prompt", &["model", "thoughtLevel", "allowedTools", "maxTurns", "timeout", "toolMode", "agentConfig"]);
    assert_variant_supports_fields(&variants, "sequentialThinking", &["model", "thoughtLevel", "allowedTools", "maxTurns", "timeout", "agentConfig"]);
    assert_variant_supports_fields(&variants, "tool", &["model", "thoughtLevel", "allowedTools", "maxTurns", "agentConfig"]);
    assert_variant_required_fields(&variants);
    assert_agent_config_fields(&variants, "prompt");
    assert_agent_config_fields(&variants, "sequentialThinking");
    assert_agent_config_fields(&variants, "tool");
}
fn definition_node_schema() -> Value {
    pipeline_schema()
        .get("properties")
        .and_then(Value::as_object)
        .and_then(|properties| properties.get("definition"))
        .cloned()
        .expect("definition property")
}

fn graft_node_schema() -> Value {
    pipeline_schema()
        .get("properties")
        .and_then(Value::as_object)
        .and_then(|properties| properties.get("graft"))
        .cloned()
        .expect("graft property")
}
fn node_variants(container: &Value) -> Vec<&Value> {
    container
        .get("properties")
        .and_then(Value::as_object)
        .and_then(|properties| properties.get("nodes"))
        .and_then(Value::as_object)
        .and_then(|nodes| nodes.get("items"))
        .and_then(Value::as_object)
        .and_then(|items| items.get("oneOf"))
        .and_then(Value::as_array)
        .expect("node variants")
        .iter()
        .collect()
}

fn pipeline_schema() -> Value {
    tool("Pipeline").expect("pipeline tool schema").input_schema
}
fn assert_variant_supports_fields(variants: &[&Value], node_type: &str, fields: &[&str]) {
    let properties = variant_properties(variants, node_type);
    for field in fields {
        assert!(
            properties.contains_key(*field),
            "{node_type} node schema missing {field}"
        );
    }
}
fn assert_variant_required_fields(variants: &[&Value]) {
    for (node_type, field) in [
        ("workflow", "type"),
        ("prompt", "type"),
        ("sequentialThinking", "type"),
        ("sequentialThinking", "totalThoughts"),
        ("tool", "type"),
    ] {
        assert!(variant_required(variants, node_type).contains(&field), "{node_type} required missing {field}");
    }
}
fn assert_agent_config_fields(variants: &[&Value], node_type: &str) {
    let agent_config = variant_properties(variants, node_type)
        .get("agentConfig")
        .and_then(Value::as_object)
        .expect("agentConfig property");
    let config_properties = agent_config
        .get("properties")
        .and_then(Value::as_object)
        .expect("agentConfig properties");

    for field in ["mode", "configOptions"] {
        assert!(
            config_properties.contains_key(field),
            "{node_type} agentConfig missing {field}"
        );
    }
}
fn variant_properties<'a>(
    variants: &'a [&Value],
    node_type: &str,
) -> &'a serde_json::Map<String, Value> {
    variants
        .iter()
        .copied()
        .find(|variant| variant_type(variant) == Some(node_type))
        .and_then(|variant| variant.get("properties"))
        .and_then(Value::as_object)
        .expect("node variant properties")
}
fn variant_required<'a>(variants: &'a [&Value], node_type: &str) -> Vec<&'a str> {
    variants
        .iter()
        .copied()
        .find(|variant| variant_type(variant) == Some(node_type))
        .and_then(|variant| variant.get("required"))
        .and_then(Value::as_array)
        .map(|values| values.iter().filter_map(Value::as_str).collect())
        .expect("node variant required")
}
fn variant_type(variant: &Value) -> Option<&str> {
    variant
        .get("properties")
        .and_then(Value::as_object)
        .and_then(|properties| properties.get("type"))
        .and_then(Value::as_object)
        .and_then(|kind| kind.get("enum"))
        .and_then(Value::as_array)
        .and_then(|values| values.first())
        .and_then(Value::as_str)
}
