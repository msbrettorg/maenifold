use crate::markdown::{Frontmatter, uri_to_path, write_markdown};
use crate::sessions::{pipeline_session_uri, workflow_session_uri};
use crate::tools::ToolContext;
use serde_yml::Value as YamlValue;
use std::collections::{BTreeMap, HashMap};

use super::test_fixtures_helpers::{
    node_status_string, pipeline_session_frontmatter, write_workflow_sessions,
};
use super::{NodeState, PipelineDefinition};

fn ensure_uri_parent_exists(context: &ToolContext, uri: &str) {
    let path = uri_to_path(uri, &context.config.memory_path).expect("resolve markdown path");
    let parent = path.parent().expect("markdown parent");
    std::fs::create_dir_all(parent).expect("create markdown parent");
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
    let pipeline_uri = pipeline_session_uri(pipeline_id);
    ensure_uri_parent_exists(context, &pipeline_uri);
    write_markdown(
        &pipeline_uri,
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
        YamlValue::String("pipeline".to_string()),
    );
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("active".to_string()),
    );
    frontmatter.insert(
        "definition".to_string(),
        serde_yml::to_value(&definition).expect("serialize definition"),
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
        serde_yml::to_value(max_concurrency).expect("yaml"),
    );

    let pipeline_uri = pipeline_session_uri(pipeline_id);
    ensure_uri_parent_exists(context, &pipeline_uri);
    write_markdown(
        &pipeline_uri,
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

        let workflow_uri = workflow_session_uri(&state.session_id);
        ensure_uri_parent_exists(context, &workflow_uri);
        write_markdown(
            &workflow_uri,
            Some(&session_frontmatter),
            "# Workflow Session\n",
            &context.config,
        )
        .expect("write node session");
    }
}
