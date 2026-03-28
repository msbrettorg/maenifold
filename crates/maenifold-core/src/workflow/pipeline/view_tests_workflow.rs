use super::*;
use crate::sessions::workflow_session_uri;

#[test]
#[serial(config)]
fn pipeline_view_returns_workflow_steps_for_active_and_completed_nodes() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let definition = PipelineDefinition {
        nodes: vec![
            workflow_node("node-done", "test-workflow-a"),
            workflow_node("node-active", "test-workflow-b"),
            workflow_node("node-pending", "test-workflow-a"),
        ],
        edges: vec![
            edge("node-done", "node-active", EdgeType::Success),
            edge("node-active", "node-pending", EdgeType::Success),
        ],
    };
    let node_states = HashMap::from([
        (
            "node-done".to_string(),
            node_state(
                "workflow-1735689600000-done",
                NodeStatus::Completed,
                1,
                Some("all done"),
            ),
        ),
        (
            "node-active".to_string(),
            node_state("workflow-1735689600000-active", NodeStatus::Active, 1, None),
        ),
        (
            "node-pending".to_string(),
            node_state(
                "workflow-1735689600000-pending",
                NodeStatus::Pending,
                0,
                None,
            ),
        ),
    ]);

    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-wf-steps",
        definition.clone(),
        node_states.clone(),
        8,
    );

    let completed_uri = workflow_session_uri("workflow-1735689600000-done");
    append_h2_section(
        &completed_uri,
        "Step 1/2",
        "{\n      \"id\": \"research\",\n      \"name\": \"Research evidence\",\n      \"description\": \"Collect the supporting material\"\n    }",
        &context.config,
    )
    .expect("append completed step 1");
    append_h2_section(
        &completed_uri,
        "Step 2/2",
        "{\n      \"id\": \"analysis\",\n      \"name\": \"Analyze findings\",\n      \"description\": \"Synthesize the research into recommendations\"\n    }",
        &context.config,
    )
    .expect("append completed step 2");
    append_h2_section(
        &completed_uri,
        "Conclusion",
        "Final summary of the workflow.",
        &context.config,
    )
    .expect("append conclusion");

    let active_uri = workflow_session_uri("workflow-1735689600000-active");
    append_h2_section(
        &active_uri,
        "Step 1/3",
        "{\n      \"id\": \"gather\",\n      \"name\": \"Gather data\",\n      \"description\": \"Collect metrics from all sources\"\n    }",
        &context.config,
    )
    .expect("append active step 1");
    append_h2_section(
        &active_uri,
        "Step 2/3",
        "{\n      \"id\": \"validate\",\n      \"name\": \"Validate input\",\n      \"description\": \"❌ Validation failed for dataset B\"\n    }",
        &context.config,
    )
    .expect("append active step 2");
    append_h2_section(
        &active_uri,
        "Step 3/3",
        "{\n      \"id\": \"transform\",\n      \"name\": \"Transform data\",\n      \"description\": \"Apply the normalized transformation\"\n    }",
        &context.config,
    )
    .expect("append active step 3");

    let result = build_pipeline_view_result(
        &context,
        "pipeline-wf-steps",
        &definition,
        &node_states,
        None,
    )
    .expect("build pipeline view");
    let nodes = result
        .get("nodes")
        .and_then(Value::as_array)
        .expect("nodes array");

    let done_node = nodes
        .iter()
        .find(|n| n["nodeId"] == "node-done")
        .expect("find node-done");
    let done_steps = done_node["workflow_steps"]
        .as_array()
        .expect("workflow_steps array for completed node");
    assert_eq!(done_steps.len(), 2);
    assert_eq!(done_steps[0]["step_name"], "Research evidence");
    assert_eq!(done_steps[0]["status"], "complete");
    assert!(
        done_steps[0]["output"]
            .as_str()
            .unwrap()
            .contains("\"research\"")
    );
    assert_eq!(done_steps[1]["step_name"], "Analyze findings");
    assert_eq!(done_steps[1]["status"], "complete");

    let active_node = nodes
        .iter()
        .find(|n| n["nodeId"] == "node-active")
        .expect("find node-active");
    let active_steps = active_node["workflow_steps"]
        .as_array()
        .expect("workflow_steps array for active node");
    assert_eq!(active_steps.len(), 3);
    assert_eq!(active_steps[0]["step_name"], "Gather data");
    assert_eq!(active_steps[0]["status"], "complete");
    assert_eq!(active_steps[1]["step_name"], "Validate input");
    assert_eq!(active_steps[1]["status"], "failed");
    assert_eq!(active_steps[2]["step_name"], "Transform data");
    assert_eq!(active_steps[2]["status"], "active");
    assert!(
        active_steps[2]["output"]
            .as_str()
            .unwrap()
            .contains("\"transform\"")
    );

    let pending_node = nodes
        .iter()
        .find(|n| n["nodeId"] == "node-pending")
        .expect("find node-pending");
    assert_eq!(pending_node["workflow_steps"], Value::Null);
}
