use crate::markdown::{Frontmatter, write_markdown};
use crate::sessions::{pipeline_session_uri, workflow_session_uri};
use crate::tools::ToolContext;
use serde_yaml::Value as YamlValue;
use std::collections::{BTreeMap, HashMap};

use super::test_fixtures_helpers::{
    node_status_string, pipeline_session_frontmatter, write_workflow_sessions,
};
use super::{EdgeType, NodeState, NodeStatus, PipelineDefinition};

#[allow(clippy::too_many_lines)]
pub(super) fn seed_pipeline_view_session(
    context: &ToolContext,
    session_id: &str,
    session_type: &str,
) -> PipelineDefinition {
    let definition = PipelineDefinition {
        nodes: vec![
            crate::workflow::pipeline::PipelineNode::Workflow {
                id: "root".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: Some("agent-root".to_string()),
                prompt: None,

                timeout: None,
            },
            crate::workflow::pipeline::PipelineNode::Workflow {
                id: "cleanup".to_string(),
                workflow_id: "test-workflow-b".to_string(),
                acp_client: Some("agent-b".to_string()),
                prompt: None,

                timeout: None,
            },
            crate::workflow::pipeline::PipelineNode::Workflow {
                id: "child".to_string(),
                workflow_id: "test-workflow-b".to_string(),
                acp_client: Some("agent-child".to_string()),
                prompt: None,

                timeout: None,
            },
        ],
        edges: vec![
            crate::workflow::pipeline::PipelineEdge {
                source: "root".to_string(),
                target: "child".to_string(),
                edge_type: EdgeType::Success,
                mapping: None,
            },
            crate::workflow::pipeline::PipelineEdge {
                source: "root".to_string(),
                target: "cleanup".to_string(),
                edge_type: EdgeType::Always,
                mapping: None,
            },
        ],
    };
    let node_states = HashMap::from([
        (
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689600000-root".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: None,
                failure_reason: None,
            },
        ),
        (
            "cleanup".to_string(),
            NodeState {
                session_id: "workflow-1735689600000-cleanup".to_string(),
                status: NodeStatus::Active,
                attempts: 2,
                thinking_session_id: None,
                output: None,
                failure_reason: None,
            },
        ),
        (
            "child".to_string(),
            NodeState {
                session_id: "workflow-1735689600000-child".to_string(),
                status: NodeStatus::Pending,
                attempts: 0,
                thinking_session_id: None,
                output: None,
                failure_reason: None,
            },
        ),
    ]);

    let frontmatter =
        pipeline_session_frontmatter(session_id, session_type, &definition, &node_states, None);
    write_markdown(
        &pipeline_session_uri(session_id),
        Some(&frontmatter),
        "# Pipeline Session\n",
        &context.config,
    )
    .expect("write pipeline session");
    write_workflow_sessions(context, session_id, &definition, &node_states);

    definition
}

pub(super) fn seed_dispatch_pipeline(
    context: &ToolContext,
    pipeline_id: &str,
    node_states: HashMap<String, NodeState>,
    max_concurrency: u64,
) -> PipelineDefinition {
    let definition = PipelineDefinition {
        nodes: vec![
            crate::workflow::pipeline::PipelineNode::Workflow {
                id: "root".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: Some("agent-root".to_string()),
                prompt: None,

                timeout: None,
            },
            crate::workflow::pipeline::PipelineNode::Workflow {
                id: "child".to_string(),
                workflow_id: "test-workflow-b".to_string(),
                acp_client: Some("agent-child".to_string()),
                prompt: None,

                timeout: None,
            },
        ],
        edges: vec![crate::workflow::pipeline::PipelineEdge {
            source: "root".to_string(),
            target: "child".to_string(),
            edge_type: EdgeType::Success,
            mapping: None,
        }],
    };

    seed_pipeline_session_with_workflows(
        context,
        pipeline_id,
        definition.clone(),
        node_states,
        max_concurrency,
    );

    definition
}

pub(super) fn seed_pipeline_session_with_workflows(
    context: &ToolContext,
    pipeline_id: &str,
    definition: PipelineDefinition,
    node_states: HashMap<String, NodeState>,
    max_concurrency: u64,
) {
    let frontmatter = pipeline_session_frontmatter(
        pipeline_id,
        "pipeline",
        &definition,
        &node_states,
        Some(max_concurrency),
    );
    write_markdown(
        &pipeline_session_uri(pipeline_id),
        Some(&frontmatter),
        "# Pipeline Session\n",
        &context.config,
    )
    .expect("write pipeline session");
    write_workflow_sessions(context, pipeline_id, &definition, &node_states);
}

/// Seed a pipeline session whose node sessions use plain frontmatter (no workflow steps).
///
/// Used in tests that exercise Prompt / SequentialThinking / Tool node dispatch, where
/// the node sessions are not full workflow sessions and must not have `workflowId`.
#[allow(clippy::too_many_lines)]
pub(super) fn seed_pipeline_session_with_node_sessions(
    context: &ToolContext,
    pipeline_id: &str,
    definition: PipelineDefinition,
    node_states: HashMap<String, NodeState>,
    max_concurrency: u64,
) {
    let node_states_yaml: BTreeMap<String, YamlValue> = node_states
        .iter()
        .map(|(key, value)| (key.clone(), serde_yaml::to_value(value).expect("yaml")))
        .collect();
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
        serde_yaml::to_value(&definition).expect("serialize definition"),
    );
    frontmatter.insert(
        "nodeStates".to_string(),
        YamlValue::Mapping(
            node_states_yaml
                .iter()
                .map(|(key, value)| (YamlValue::String(key.clone()), value.clone()))
                .collect(),
        ),
    );
    frontmatter.insert(
        "maxConcurrency".to_string(),
        serde_yaml::to_value(max_concurrency).expect("yaml"),
    );

    write_markdown(
        &pipeline_session_uri(pipeline_id),
        Some(&frontmatter),
        "# Pipeline Session\n",
        &context.config,
    )
    .expect("write pipeline session");

    for node in &definition.nodes {
        let state = node_states.get(node.id()).expect("node state");
        let mut session_frontmatter = Frontmatter::new();
        session_frontmatter.insert(
            "title".to_string(),
            YamlValue::String(format!("Workflow Session {}", state.session_id)),
        );
        session_frontmatter.insert(
            "permalink".to_string(),
            YamlValue::String(state.session_id.clone()),
        );
        session_frontmatter.insert(
            "type".to_string(),
            YamlValue::String("workflow".to_string()),
        );
        session_frontmatter.insert(
            "status".to_string(),
            YamlValue::String(node_status_string(&state.status).to_string()),
        );
        session_frontmatter.insert(
            "pipelineId".to_string(),
            YamlValue::String(pipeline_id.to_string()),
        );
        session_frontmatter.insert(
            "nodeId".to_string(),
            YamlValue::String(node.id().to_string()),
        );
        if let Some(workflow_id) = node.workflow_id() {
            session_frontmatter.insert(
                "workflowId".to_string(),
                YamlValue::String(workflow_id.to_string()),
            );
        }

        write_markdown(
            &workflow_session_uri(&state.session_id),
            Some(&session_frontmatter),
            "# Workflow Session\n",
            &context.config,
        )
        .expect("write node session");
    }
}
