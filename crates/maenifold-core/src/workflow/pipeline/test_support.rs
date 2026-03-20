use crate::config::Config;
use crate::markdown::{Frontmatter, write_markdown};
use crate::sessions::{pipeline_session_uri, workflow_session_uri};
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use serde_json::json;
use serde_yaml::Value as YamlValue;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use tempfile::TempDir;

use super::{EdgeType, NodeState, NodeStatus, PipelineDefinition};

pub(super) struct ConfigResetGuard;

impl Drop for ConfigResetGuard {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

pub(super) fn pipeline_test_context(
    output_format: OutputFormat,
) -> (TempDir, ToolContext, ConfigResetGuard) {
    let tmp = TempDir::new().expect("temp dir");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let config = Config::current();
    config.ensure_directories().expect("ensure directories");
    fs::create_dir_all(config.assets_path.join("workflows")).expect("create workflows dir");
    fs::create_dir_all(config.assets_path.join("usage/tools")).expect("create usage dir");
    fs::write(
        config.assets_path.join("usage/tools/pipeline.md"),
        "# Pipeline\n\nCreates and manages pipeline sessions composed of workflow nodes.\n\n## Operations\n\n- `definition`: create a pipeline session from `nodes` and `edges`.\n- `run`: start autonomous execution for a pipeline session.\n- `pause`: pause a running pipeline.\n- `resume`: resume a paused pipeline.\n- `cancel`: cancel a running pipeline.\n- `view`: inspect the current pipeline state.\n- `dispatch`: claim ready nodes for execution.\n- `restart`: recreate a node session for an abandoned or stuck node.\n- `rewind`: reset selected node IDs and their downstream nodes.\n- `delete`: permanently delete the pipeline session and associated workflow sessions.\n- `graft`: add nodes and edges to an existing pipeline.\n- `join`: merge another pipeline into this pipeline.\n- `fork`: clone a subgraph into a new pipeline session.\n",
    )
    .expect("write help");

    for workflow_id in ["test-workflow-a", "test-workflow-b"] {
        fs::write(
            config
                .assets_path
                .join("workflows")
                .join(format!("{workflow_id}.json")),
            serde_json::to_string_pretty(&json!({
                "steps": [
                    {
                        "id": "step-1",
                        "name": "Step 1",
                        "description": format!("Run [[{workflow_id}]]")
                    }
                ]
            }))
            .expect("serialize workflow"),
        )
        .expect("write workflow");
    }

    (
        tmp,
        ToolContext::new(config, output_format),
        ConfigResetGuard,
    )
}

pub(super) fn seed_pipeline_view_session(
    context: &ToolContext,
    session_id: &str,
    session_type: &str,
) -> PipelineDefinition {
    let definition = PipelineDefinition {
        nodes: vec![
            crate::workflow::pipeline::PipelineNode {
                id: "root".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: None,
            },
            crate::workflow::pipeline::PipelineNode {
                id: "cleanup".to_string(),
                workflow_id: "test-workflow-b".to_string(),
                acp_client: Some("agent-b".to_string()),
            },
            crate::workflow::pipeline::PipelineNode {
                id: "child".to_string(),
                workflow_id: "test-workflow-b".to_string(),
                acp_client: None,
            },
        ],
        edges: vec![
            crate::workflow::pipeline::PipelineEdge {
                source: "root".to_string(),
                target: "child".to_string(),
                edge_type: EdgeType::Success,
            },
            crate::workflow::pipeline::PipelineEdge {
                source: "root".to_string(),
                target: "cleanup".to_string(),
                edge_type: EdgeType::Always,
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
            },
        ),
        (
            "cleanup".to_string(),
            NodeState {
                session_id: "workflow-1735689600000-cleanup".to_string(),
                status: NodeStatus::Active,
                attempts: 2,
            },
        ),
        (
            "child".to_string(),
            NodeState {
                session_id: "workflow-1735689600000-child".to_string(),
                status: NodeStatus::Pending,
                attempts: 0,
            },
        ),
    ]);

    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String(format!("Pipeline Session {session_id}")),
    );
    frontmatter.insert(
        "permalink".to_string(),
        YamlValue::String(session_id.to_string()),
    );
    frontmatter.insert(
        "type".to_string(),
        YamlValue::String(session_type.to_string()),
    );
    frontmatter.insert(
        "definition".to_string(),
        serde_yaml::to_value(&definition).expect("serialize definition"),
    );
    frontmatter.insert(
        "nodeStates".to_string(),
        serde_yaml::to_value(&node_states).expect("serialize node states"),
    );

    write_markdown(
        &pipeline_session_uri(session_id),
        Some(&frontmatter),
        "# Pipeline Session\n",
        &context.config,
    )
    .expect("write pipeline session");

    for (node_id, state) in &node_states {
        let workflow_id = definition
            .nodes
            .iter()
            .find(|node| node.id == *node_id)
            .expect("workflow node")
            .workflow_id
            .clone();
        let mut workflow_frontmatter = crate::workflow::initial_workflow_frontmatter(
            &state.session_id,
            std::slice::from_ref(&workflow_id),
            match state.status {
                NodeStatus::Pending => "pending",
                NodeStatus::Ready => "ready",
                NodeStatus::Active => "active",
                NodeStatus::Completed => "completed",
                NodeStatus::Abandoned => "abandoned",
                NodeStatus::Stuck => "stuck",
            },
        );
        workflow_frontmatter.insert(
            "pipelineId".to_string(),
            YamlValue::String(session_id.to_string()),
        );
        workflow_frontmatter.insert("nodeId".to_string(), YamlValue::String(node_id.clone()));
        workflow_frontmatter.insert("workflowId".to_string(), YamlValue::String(workflow_id));
        write_markdown(
            &workflow_session_uri(&state.session_id),
            Some(&workflow_frontmatter),
            "# Workflow Session\n",
            &context.config,
        )
        .expect("write workflow session");
    }

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
            crate::workflow::pipeline::PipelineNode {
                id: "root".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: None,
            },
            crate::workflow::pipeline::PipelineNode {
                id: "child".to_string(),
                workflow_id: "test-workflow-b".to_string(),
                acp_client: None,
            },
        ],
        edges: vec![crate::workflow::pipeline::PipelineEdge {
            source: "root".to_string(),
            target: "child".to_string(),
            edge_type: EdgeType::Success,
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
    let node_status_string = |status: &NodeStatus| match status {
        NodeStatus::Pending => "pending",
        NodeStatus::Active => "active",
        NodeStatus::Completed => "completed",
        NodeStatus::Abandoned => "abandoned",
        NodeStatus::Ready => "ready",
        NodeStatus::Stuck => "stuck",
    };

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
        let state = node_states.get(&node.id).expect("node state");
        let uri = workflow_session_uri(&state.session_id);
        let mut wf_fm = crate::workflow::initial_workflow_frontmatter(
            &state.session_id,
            std::slice::from_ref(&node.workflow_id),
            node_status_string(&state.status),
        );
        wf_fm.insert(
            "pipelineId".to_string(),
            YamlValue::String(pipeline_id.to_string()),
        );
        wf_fm.insert("nodeId".to_string(), YamlValue::String(node.id.to_string()));
        wf_fm.insert(
            "workflowId".to_string(),
            YamlValue::String(node.workflow_id.to_string()),
        );
        write_markdown(&uri, Some(&wf_fm), "# Workflow Session\n", &context.config)
            .expect("write workflow session");
    }
}
