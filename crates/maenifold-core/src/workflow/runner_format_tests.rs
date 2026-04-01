use super::format_workflow_step;
use serde_json::{Value, json};

#[test]
fn format_workflow_step_produces_pretty_json_with_indent() {
    let step = json!({
        "id": "step-1",
        "name": "Step 1",
        "description": "A test step for [[workflow]] coverage"
    });
    let output = format_workflow_step(&step).expect("format step");

    assert!(
        output.lines().count() > 1,
        "pretty JSON must be multi-line, got: {output:?}"
    );
    assert!(
        output.contains("\n      \"id\""),
        "properties must be indented 6 spaces: {output:?}"
    );

    let parsed: Value = serde_json::from_str(&output).expect("valid JSON roundtrip");
    assert_eq!(parsed["id"], "step-1");
    assert_eq!(parsed["name"], "Step 1");
}

#[test]
fn format_workflow_step_empty_object_is_braces() {
    let step = json!({});
    let output = format_workflow_step(&step).expect("format empty step");
    assert_eq!(output, "{}", "empty object must be compact: {output:?}");
}

#[test]
fn format_workflow_step_nested_object_has_deeper_indent() {
    let step = json!({
        "id": "step-2",
        "name": "Step 2",
        "metadata": {
            "sequentialThinking": true,
            "tags": ["research", "analysis"]
        }
    });
    let output = format_workflow_step(&step).expect("format nested step");

    assert!(
        output.lines().count() > 1,
        "nested pretty JSON must be multi-line, got: {output:?}"
    );
    assert!(
        output.contains("\n        \"sequentialThinking\""),
        "nested properties must be indented 8+ spaces: {output:?}"
    );

    let parsed: Value = serde_json::from_str(&output).expect("valid JSON roundtrip");
    assert_eq!(parsed["metadata"]["sequentialThinking"], true);
}

#[test]
fn format_workflow_step_closing_brace_at_four_spaces() {
    let step = json!({ "id": "step-1" });
    let output = format_workflow_step(&step).expect("format step");

    assert_eq!(
        output.lines().last().unwrap(),
        "    }",
        "closing brace at 4-space indent: {output:?}"
    );
}

#[test]
fn format_workflow_step_with_multiline_description_roundtrips() {
    let step = json!({
        "id": "step-3",
        "name": "Step 3",
        "description": "Line 1\nLine 2\nLine 3"
    });
    let output = format_workflow_step(&step).expect("format multiline desc step");

    let parsed: Value = serde_json::from_str(&output).expect("valid JSON roundtrip");
    assert_eq!(parsed["description"], "Line 1\nLine 2\nLine 3");
}
