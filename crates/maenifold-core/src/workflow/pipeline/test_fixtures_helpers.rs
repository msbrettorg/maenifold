use crate::markdown::{Frontmatter, write_markdown};
use crate::sessions::workflow_session_uri;
use crate::tools::ToolContext;
use serde_yml::Value as YamlValue;
use std::collections::{BTreeMap, HashMap};

use super::{NodeState, NodeStatus, PipelineDefinition};

pub(super) fn node_status_string(status: &NodeStatus) -> &'static str {
    match status {
        NodeStatus::Pending => "pending",
        NodeStatus::Active => "active",
        NodeStatus::Completed => "completed",
        NodeStatus::Skipped => "skipped",
        NodeStatus::Abandoned => "abandoned",
        NodeStatus::Ready => "ready",
        NodeStatus::Stuck => "stuck",
        NodeStatus::Failed => "failed",
        NodeStatus::Retrying => "retrying",
    }
}

pub(super) fn pipeline_session_frontmatter(
    pipeline_id: &str,
    session_type: &str,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
    max_concurrency: Option<u64>,
) -> Frontmatter {
    let node_states_yaml: BTreeMap<String, YamlValue> = node_states
        .iter()
        .map(|(key, value)| (key.clone(), serde_yml::to_value(value).expect("yaml")))
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
        YamlValue::String(session_type.to_string()),
    );
    frontmatter.insert(
        "definition".to_string(),
        serde_yml::to_value(definition).expect("serialize definition"),
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

    if let Some(max_concurrency) = max_concurrency {
        frontmatter.insert(
            "status".to_string(),
            YamlValue::String("active".to_string()),
        );
        frontmatter.insert(
            "maxConcurrency".to_string(),
            serde_yml::to_value(max_concurrency).expect("yaml"),
        );
    }

    frontmatter
}

pub(super) fn write_workflow_sessions(
    context: &ToolContext,
    pipeline_id: &str,
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
) {
    for node in &definition.nodes {
        let state = node_states.get(node.id()).expect("node state");
        let workflow_id = node.effective_workflow_id();
        let mut workflow_frontmatter = crate::workflow::initial_workflow_frontmatter(
            &state.session_id,
            std::slice::from_ref(&workflow_id),
            node_status_string(&state.status),
            None,
        );
        workflow_frontmatter.insert(
            "pipelineId".to_string(),
            YamlValue::String(pipeline_id.to_string()),
        );
        workflow_frontmatter.insert(
            "nodeId".to_string(),
            YamlValue::String(node.id().to_string()),
        );
        workflow_frontmatter.insert(
            "workflowId".to_string(),
            YamlValue::String(workflow_id.to_string()),
        );
        write_markdown(
            &workflow_session_uri(&state.session_id),
            Some(&workflow_frontmatter),
            "# Workflow Session\n",
            &context.config,
        )
        .expect("write workflow session");
    }
}
