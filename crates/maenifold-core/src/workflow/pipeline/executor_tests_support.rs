use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;

use super::session_io::frontmatter_value;
use crate::markdown::{Frontmatter, read_markdown, write_markdown};
use crate::sessions::{pipeline_session_uri, workflow_session_uri};
use crate::tools::ToolContext;
use crate::workflow::initial_workflow_frontmatter;
pub(super) use crate::workflow::pipeline::test_support::{
    pipeline_test_context, seed_pipeline_session_with_workflows,
};
use crate::workflow::pipeline::{NodeState, NodeStatus, PipelineDefinition, PipelineNode};

pub(super) fn source_tool_node(id: &str) -> PipelineNode {
    PipelineNode::Tool {
        id: id.to_string(),
        tool_name: "GetConfig".to_string(),
        tool_args: None,
        agent: None,
        agent_config: None,
        model: None,
        thought_level: None,
        allowed_tools: None,
        max_turns: None,
    }
}

pub(super) fn target_tool_node(tool_args: Option<Value>) -> PipelineNode {
    PipelineNode::Tool {
        id: "tool".to_string(),
        tool_name: "GetConfig".to_string(),
        tool_args,
        agent: None,
        agent_config: None,
        model: None,
        thought_level: None,
        allowed_tools: None,
        max_turns: None,
    }
}

pub(super) fn completed_node_state(
    id: &str,
    session_id: &str,
    output: Value,
) -> (String, NodeState) {
    (
        id.to_string(),
        NodeState {
            session_id: session_id.to_string(),
            status: NodeStatus::Completed,
            attempts: 1,
            thinking_session_id: None,
            output: Some(output.to_string()),
        },
    )
}

pub(super) fn pending_node_state(id: &str, session_id: &str) -> (String, NodeState) {
    (
        id.to_string(),
        NodeState {
            session_id: session_id.to_string(),
            status: NodeStatus::Pending,
            attempts: 0,
            thinking_session_id: None,
            output: None,
        },
    )
}

pub(super) fn pipeline_node(id: &str, workflow_id: &str) -> PipelineNode {
    PipelineNode::Workflow {
        id: id.to_string(),
        workflow_id: workflow_id.to_string(),
        acp_client: Some(format!("test-agent-{id}")),
        prompt: None,

        timeout: None,
    }
}

pub(super) fn write_workflow_definition(assets_path: &Path, workflow_id: &str) {
    let workflows_dir = assets_path.join("workflows");
    fs::create_dir_all(&workflows_dir).expect("workflows dir");
    let definition = json!({
        "steps": [{
            "id": "step-1",
            "name": "Step 1",
            "description": "Execute [[task]]"
        }]
    });
    fs::write(
        workflows_dir.join(format!("{workflow_id}.json")),
        serde_json::to_vec_pretty(&definition).expect("workflow json"),
    )
    .expect("write workflow definition");
}

pub(super) fn seed_pipeline_session_with_nodes(
    context: &ToolContext,
    pipeline_id: &str,
    definition: PipelineDefinition,
    node_states: HashMap<String, NodeState>,
    max_concurrency: u64,
) {
    let frontmatter =
        pipeline_session_frontmatter(pipeline_id, &definition, &node_states, max_concurrency);
    write_markdown(
        &pipeline_session_uri(pipeline_id),
        Some(&frontmatter),
        "# Pipeline Session\n",
        &context.config,
    )
    .expect("write pipeline session");

    for node in &definition.nodes {
        let state = node_states.get(node.id()).expect("node state");
        let workflow_frontmatter =
            workflow_session_frontmatter(pipeline_id, node, &state.session_id, &state.status);
        write_markdown(
            &workflow_session_uri(&state.session_id),
            Some(&workflow_frontmatter),
            "# Workflow Session\n",
            &context.config,
        )
        .expect("write node session");
    }
}

fn pipeline_session_frontmatter(
    pipeline_id: &str,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    max_concurrency: u64,
) -> Frontmatter {
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String(format!("Pipeline Session {pipeline_id}")),
    );
    frontmatter.insert(
        "permalink".to_string(),
        YamlValue::String(pipeline_id.to_string()),
    );
    frontmatter.insert(
        "type".to_string(),
        YamlValue::String("pipeline".to_string()),
    );
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("active".to_string()),
    );
    frontmatter.insert(
        "definition".to_string(),
        serde_yaml::to_value(definition).expect("serialize definition"),
    );
    frontmatter.insert(
        "nodeStates".to_string(),
        serde_yaml::to_value(node_states).expect("serialize node states"),
    );
    frontmatter.insert(
        "maxConcurrency".to_string(),
        serde_yaml::to_value(max_concurrency).expect("serialize concurrency"),
    );
    frontmatter
}

fn workflow_session_frontmatter(
    pipeline_id: &str,
    node: &PipelineNode,
    session_id: &str,
    status: &NodeStatus,
) -> Frontmatter {
    let workflow_ref = node
        .workflow_id()
        .map(str::to_string)
        .unwrap_or_else(|| format!("pipeline-{}-{}", node.node_type_name(), node.id()));
    let mut frontmatter = initial_workflow_frontmatter(
        session_id,
        std::slice::from_ref(&workflow_ref),
        node_status_name(status),
    );
    frontmatter.insert(
        "pipelineId".to_string(),
        YamlValue::String(pipeline_id.to_string()),
    );
    frontmatter.insert(
        "nodeId".to_string(),
        YamlValue::String(node.id().to_string()),
    );
    frontmatter.insert("workflowId".to_string(), YamlValue::String(workflow_ref));
    frontmatter
}

pub(super) fn read_pipeline_node_states(
    context: &ToolContext,
    pipeline_id: &str,
) -> HashMap<String, NodeState> {
    let (frontmatter, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read pipeline");
    let frontmatter = frontmatter.expect("pipeline frontmatter");
    frontmatter_value(&frontmatter, "nodeStates").expect("node states")
}

pub(super) fn read_workflow_status(context: &ToolContext, session_id: &str) -> Option<String> {
    let (frontmatter, _, _) =
        read_markdown(&workflow_session_uri(session_id), &context.config).expect("read workflow");
    frontmatter
        .expect("workflow frontmatter")
        .get("status")
        .and_then(YamlValue::as_str)
        .map(str::to_string)
}

pub(super) fn read_workflow_conclusion(context: &ToolContext, session_id: &str) -> Option<String> {
    let (frontmatter, _, _) =
        read_markdown(&workflow_session_uri(session_id), &context.config).expect("read workflow");
    frontmatter
        .expect("workflow frontmatter")
        .get("conclusion")
        .and_then(YamlValue::as_str)
        .map(str::to_string)
}

fn node_status_name(status: &NodeStatus) -> &'static str {
    match status {
        NodeStatus::Pending => "pending",
        NodeStatus::Ready => "ready",
        NodeStatus::Active => "active",
        NodeStatus::Completed => "completed",
        NodeStatus::Abandoned => "abandoned",
        NodeStatus::Stuck => "stuck",
    }
}
