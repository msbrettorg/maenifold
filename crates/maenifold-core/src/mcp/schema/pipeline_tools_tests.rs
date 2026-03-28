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
        assert!(
            property
                .get("description")
                .and_then(Value::as_str)
                .is_some_and(|description| !description.is_empty())
        );
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
