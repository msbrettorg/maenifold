use std::collections::HashMap;

use serde_json::{Value as JsonValue, json};

use crate::util::json::OutputFormat;
use crate::workflow::runner::{WorkflowResult, WorkflowRunStatus};

use super::executor_apply::apply_node_outcome;
use super::executor_helpers::DispatchedNode;
use super::executor_nodes::{NodeExecutionRequest, NodeTaskResult, classify_node_result};
use super::executor_orchestration::ExecutionCounts;
use super::mapping_apply::mapped_tool_args;
use super::test_support::{pipeline_test_context, seed_pipeline_session_with_node_sessions};
use super::{EdgeDataMode, EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineEdge};

fn output_schema() -> JsonValue {
    json!({"type":"object","required":["verdict"],"properties":{"verdict":{"enum":["PASS","FAIL"]}}})
}

fn workflow_node(value: JsonValue) -> super::PipelineNode {
    serde_json::from_value(value).expect("workflow node should deserialize")
}

fn schema_workflow(id: &str) -> super::PipelineNode {
    workflow_node(json!({
        "type":"workflow",
        "id":id,
        "workflowId":"crta",
        "acpClient":"copilot",
        "outputSchema":output_schema()
    }))
}

fn legacy_workflow(id: &str) -> super::PipelineNode {
    workflow_node(json!({
        "type":"workflow",
        "id":id,
        "workflowId":"workflow-dispatch",
        "acpClient":"copilot"
    }))
}

fn definition(nodes: Vec<super::PipelineNode>, edges: Vec<PipelineEdge>) -> PipelineDefinition {
    PipelineDefinition {
        nodes,
        edges,
        inputs: None,
        input_schema: None,
        template_id: None,
    }
}

fn node_state(id: &str, status: NodeStatus, output: Option<&str>) -> NodeState {
    let status = match status {
        NodeStatus::Pending => "pending",
        NodeStatus::Ready => "ready",
        NodeStatus::Active => "active",
        NodeStatus::Completed => "completed",
        NodeStatus::Skipped => "skipped",
        NodeStatus::Abandoned => "abandoned",
        NodeStatus::Stuck => "stuck",
        NodeStatus::Failed => "failed",
        NodeStatus::Retrying => "retrying",
    };
    let output = output
        .map(|value| format!("output: {value}\n"))
        .unwrap_or_default();
    serde_yml::from_str(&format!(
        "sessionId: workflow-1735689604000-{id}\nstatus: {status}\nattempts: 1\n{output}"
    ))
    .expect("node state should deserialize")
}

fn build_request_for(node: &super::PipelineNode) -> NodeExecutionRequest {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    super::executor_nodes::build_node_execution_request(
        &context,
        &definition(vec![node.clone()], vec![]),
        &HashMap::new(),
        node,
        &DispatchedNode {
            node_id: node.id().to_string(),
            session_id: Some(format!("workflow-1735689604000-{}", node.id())),
        },
        "",
        &serde_json::Map::new(),
    )
    .expect("workflow execution request should build")
}

fn node_states_from_session(
    context: &crate::tools::ToolContext,
    session_id: &str,
) -> HashMap<String, NodeState> {
    let (_, frontmatter, _) =
        super::session_read::read_pipeline_session(context, session_id).expect("read pipeline");
    super::session_io::frontmatter_value(&frontmatter, "nodeStates").expect("node states")
}

#[test]
fn workflow_node_output_schema_contract_is_in_dispatch_session_config() {
    let NodeExecutionRequest::Workflow { session_config, .. } =
        build_request_for(&schema_workflow("crta"))
    else {
        panic!("schema workflow should build a workflow execution request");
    };

    assert_eq!(session_config.output_schema, Some(output_schema()));
    assert!(
        session_config
            .allowed_tools
            .as_deref()
            .is_some_and(|tools| tools.iter().any(|tool| tool == "complete_node")),
        "workflow output_schema dispatch must advertise complete_node"
    );
    assert_eq!(
        session_config
            .config_options
            .unwrap_or_default()
            .get("outputSchemaRepairTurns"),
        Some(&json!(1))
    );
}

#[test]
fn workflow_without_output_schema_keeps_legacy_dispatch_contract() {
    let NodeExecutionRequest::Workflow { session_config, .. } =
        build_request_for(&legacy_workflow("dispatch"))
    else {
        panic!("legacy workflow should build a workflow execution request");
    };

    assert_eq!(session_config.output_schema, None);
    assert!(
        !session_config
            .allowed_tools
            .unwrap_or_default()
            .iter()
            .any(|tool| tool == "complete_node")
    );
    assert!(
        !session_config
            .config_options
            .unwrap_or_default()
            .contains_key("outputSchemaRepairTurns")
    );
}

#[test]
fn workflow_schema_completion_projects_structured_args_into_binding_output_artifact() {
    let (_tmp, context, guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-workflow-schema-capture";
    let def = definition(
        vec![
            schema_workflow("crta"),
            workflow_node(json!({
                "type":"prompt",
                "id":"target",
                "prompt":"consume verdict",
                "agent":"copilot"
            })),
        ],
        vec![PipelineEdge {
            source: "crta".to_string(),
            target: "target".to_string(),
            edge_type: EdgeType::Success,
            mode: EdgeDataMode::Binding,
            mapping: Some(HashMap::from([(
                "decision".to_string(),
                "verdict".to_string(),
            )])),
        }],
    );
    seed_pipeline_session_with_node_sessions(
        &context,
        session_id,
        def.clone(),
        HashMap::from([
            (
                "crta".to_string(),
                node_state("crta", NodeStatus::Active, None),
            ),
            (
                "target".to_string(),
                node_state("target", NodeStatus::Pending, None),
            ),
        ]),
        1,
    );

    let outcome = classify_node_result(
        "crta",
        NodeTaskResult::Workflow(Ok(WorkflowResult {
            session_id: "workflow-1735689604000-crta".to_string(),
            acp_session_id: Some("acp-workflow-capture".to_string()),
            // normalize_workflow_prompt_response is responsible for projecting
            // AgentResponse.complete_node_args into WorkflowResult.conclusion as
            // a raw JSON string. Mock that post-capture shape here so this test
            // exercises the apply / projection seam end-to-end.
            conclusion: Some(r#"{"verdict":"PASS"}"#.to_string()),
            steps_completed: 6,
            status: WorkflowRunStatus::Completed,
        })),
    );
    let mut counts = ExecutionCounts::default();
    let apply_result = apply_node_outcome(&context, session_id, "crta", outcome, &mut counts);

    let states = node_states_from_session(&context, session_id);
    drop(guard);
    assert!(
        apply_result.is_ok(),
        "workflow complete_node args should populate output artifact: {:?}",
        apply_result
    );
    assert_eq!(
        states["crta"].output_artifact,
        Some(json!({"verdict":"PASS"}))
    );
    let mapped = mapped_tool_args(&def, &states, "target", None, &json!({}))
        .expect("binding edge should consume workflow output artifact");
    assert_eq!(mapped, Some(json!({"decision":"PASS"})));
}
