use crate::error::{MaenifoldError, Result};
use petgraph::algo::is_cyclic_directed;
use std::collections::HashSet;

mod dispatch;
mod engine;
pub(super) mod executor;
mod handler;
mod session_io;
mod supervisor;
#[cfg(test)]
mod test_support;
mod types;
mod update;
mod view;

pub use engine::{compute_pipeline_status, compute_ready_set};
pub use handler::pipeline;
pub use types::{EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode};

pub(super) fn workflow_id_for_node(node: &PipelineNode) -> Result<&str> {
    node.workflow_id().ok_or_else(|| {
        MaenifoldError::Message(format!(
            "ERROR: Pipeline node '{}' of type '{}' does not have a workflowId",
            node.id(),
            node.node_type_name()
        ))
    })
}

fn validate_non_empty_node_field(node_id: &str, field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Pipeline node '{node_id}' field '{field}' must not be empty"
        )));
    }

    Ok(())
}

fn validate_node_semantics(node: &PipelineNode) -> Result<()> {
    match node {
        PipelineNode::Workflow {
            id, workflow_id, ..
        } => validate_non_empty_node_field(id, "workflowId", workflow_id),
        PipelineNode::Prompt {
            id, prompt, agent, ..
        } => {
            validate_non_empty_node_field(id, "prompt", prompt)?;
            validate_non_empty_node_field(id, "agent", agent)
        }
        PipelineNode::SequentialThinking {
            id,
            topic,
            total_thoughts,
            agent,
            ..
        } => {
            validate_non_empty_node_field(id, "topic", topic)?;
            validate_non_empty_node_field(id, "agent", agent)?;

            if *total_thoughts == 0 {
                return Err(MaenifoldError::InvalidArgument(format!(
                    "Pipeline node '{id}' field 'totalThoughts' must be greater than 0"
                )));
            }

            Ok(())
        }
    }
}

pub fn validate_pipeline(definition: &PipelineDefinition) -> Result<()> {
    if definition.nodes.is_empty() {
        return Err(MaenifoldError::InvalidArgument(
            "Pipeline definition must contain at least one node".to_string(),
        ));
    }

    let mut node_ids = HashSet::with_capacity(definition.nodes.len());

    for node in &definition.nodes {
        if !node_ids.insert(node.id()) {
            return Err(MaenifoldError::InvalidArgument(format!(
                "Pipeline contains duplicate node ID '{}'",
                node.id()
            )));
        }

        validate_node_semantics(node)?;
    }

    let mut edge_pairs = HashSet::with_capacity(definition.edges.len());

    for edge in &definition.edges {
        if !node_ids.contains(edge.source.as_str()) {
            return Err(MaenifoldError::InvalidArgument(format!(
                "Pipeline edge references unknown source node '{}'",
                edge.source
            )));
        }

        if !node_ids.contains(edge.target.as_str()) {
            return Err(MaenifoldError::InvalidArgument(format!(
                "Pipeline edge references unknown target node '{}'",
                edge.target
            )));
        }

        if !edge_pairs.insert((edge.source.as_str(), edge.target.as_str())) {
            return Err(MaenifoldError::InvalidArgument(format!(
                "Pipeline contains duplicate edge '{} -> {}'",
                edge.source, edge.target
            )));
        }
    }

    if is_cyclic_directed(&definition.build_graph()) {
        return Err(MaenifoldError::InvalidArgument(
            "Pipeline definition contains a cycle".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_definition() -> PipelineDefinition {
        PipelineDefinition {
            nodes: vec![
                PipelineNode::Workflow {
                    id: "a".to_string(),
                    workflow_id: "workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                },
                PipelineNode::Workflow {
                    id: "b".to_string(),
                    workflow_id: "workflow-b".to_string(),
                    acp_client: Some("client-b".to_string()),
                    prompt: None,
                },
                PipelineNode::Workflow {
                    id: "c".to_string(),
                    workflow_id: "workflow-c".to_string(),
                    acp_client: None,
                    prompt: None,
                },
            ],
            edges: vec![
                PipelineEdge {
                    source: "a".to_string(),
                    target: "b".to_string(),
                    edge_type: EdgeType::Success,
                },
                PipelineEdge {
                    source: "b".to_string(),
                    target: "c".to_string(),
                    edge_type: EdgeType::Always,
                },
            ],
        }
    }

    fn mixed_definition() -> PipelineDefinition {
        PipelineDefinition {
            nodes: vec![
                PipelineNode::Workflow {
                    id: "workflow".to_string(),
                    workflow_id: "workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                },
                PipelineNode::Prompt {
                    id: "prompt".to_string(),
                    prompt: "Summarize the workflow output".to_string(),
                    agent: "prompt-agent".to_string(),
                    tool_mode: "normal".to_string(),
                },
                PipelineNode::SequentialThinking {
                    id: "thinking".to_string(),
                    topic: "Pipeline validation".to_string(),
                    total_thoughts: 3,
                    analysis_type: Some("analysis".to_string()),
                    agent: "thinking-agent".to_string(),
                },
            ],
            edges: vec![
                PipelineEdge {
                    source: "workflow".to_string(),
                    target: "prompt".to_string(),
                    edge_type: EdgeType::Success,
                },
                PipelineEdge {
                    source: "prompt".to_string(),
                    target: "thinking".to_string(),
                    edge_type: EdgeType::Success,
                },
            ],
        }
    }

    #[test]
    fn validate_pipeline_rejects_empty_node_list() {
        let definition = PipelineDefinition {
            nodes: vec![],
            edges: vec![],
        };

        let error = validate_pipeline(&definition).expect_err("expected empty pipeline to fail");
        assert!(error.to_string().contains("must contain at least one node"));
    }

    #[test]
    fn validate_pipeline_rejects_duplicate_node_ids() {
        let definition = PipelineDefinition {
            nodes: vec![
                PipelineNode::Workflow {
                    id: "dup".to_string(),
                    workflow_id: "workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                },
                PipelineNode::Workflow {
                    id: "dup".to_string(),
                    workflow_id: "workflow-b".to_string(),
                    acp_client: None,
                    prompt: None,
                },
            ],
            edges: vec![],
        };

        let error = validate_pipeline(&definition).expect_err("expected duplicate IDs to fail");
        assert!(error.to_string().contains("duplicate node ID 'dup'"));
    }

    #[test]
    fn validate_pipeline_rejects_unknown_edge_source() {
        let mut definition = sample_definition();
        definition.edges.push(PipelineEdge {
            source: "missing".to_string(),
            target: "a".to_string(),
            edge_type: EdgeType::Success,
        });

        let error = validate_pipeline(&definition).expect_err("expected missing source to fail");
        assert!(error.to_string().contains("unknown source node 'missing'"));
    }

    #[test]
    fn validate_pipeline_rejects_unknown_edge_target() {
        let mut definition = sample_definition();
        definition.edges.push(PipelineEdge {
            source: "a".to_string(),
            target: "missing".to_string(),
            edge_type: EdgeType::Always,
        });

        let error = validate_pipeline(&definition).expect_err("expected missing target to fail");
        assert!(error.to_string().contains("unknown target node 'missing'"));
    }

    #[test]
    fn validate_pipeline_detects_cycle() {
        let definition = PipelineDefinition {
            nodes: vec![
                PipelineNode::Workflow {
                    id: "a".to_string(),
                    workflow_id: "workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                },
                PipelineNode::Workflow {
                    id: "b".to_string(),
                    workflow_id: "workflow-b".to_string(),
                    acp_client: None,
                    prompt: None,
                },
            ],
            edges: vec![
                PipelineEdge {
                    source: "a".to_string(),
                    target: "b".to_string(),
                    edge_type: EdgeType::Success,
                },
                PipelineEdge {
                    source: "b".to_string(),
                    target: "a".to_string(),
                    edge_type: EdgeType::Always,
                },
            ],
        };

        let error = validate_pipeline(&definition).expect_err("expected cycle to fail");
        assert!(error.to_string().contains("contains a cycle"));
    }

    #[test]
    fn validate_pipeline_rejects_duplicate_edge_pair() {
        let definition = PipelineDefinition {
            nodes: vec![
                PipelineNode::Workflow {
                    id: "a".to_string(),
                    workflow_id: "workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                },
                PipelineNode::Workflow {
                    id: "b".to_string(),
                    workflow_id: "workflow-b".to_string(),
                    acp_client: None,
                    prompt: None,
                },
            ],
            edges: vec![
                PipelineEdge {
                    source: "a".to_string(),
                    target: "b".to_string(),
                    edge_type: EdgeType::Success,
                },
                PipelineEdge {
                    source: "a".to_string(),
                    target: "b".to_string(),
                    edge_type: EdgeType::Always,
                },
            ],
        };

        let error =
            validate_pipeline(&definition).expect_err("expected duplicate edge pair to fail");
        assert!(error.to_string().contains("duplicate edge 'a -> b'"));
    }

    #[test]
    fn validate_pipeline_accepts_mixed_edge_types_when_acyclic() {
        let definition = sample_definition();

        validate_pipeline(&definition).expect("valid pipeline should pass validation");
    }

    #[test]
    fn validate_pipeline_accepts_all_supported_node_types() {
        let definition = mixed_definition();

        validate_pipeline(&definition).expect("mixed valid node types should pass validation");
    }

    #[test]
    fn validate_pipeline_rejects_invalid_type_specific_fields() {
        let cases = vec![
            (
                PipelineNode::Workflow {
                    id: "workflow-node".to_string(),
                    workflow_id: "   ".to_string(),
                    acp_client: None,
                    prompt: None,
                },
                "workflow-node",
                "workflowId",
            ),
            (
                PipelineNode::Prompt {
                    id: "prompt-node".to_string(),
                    prompt: "   ".to_string(),
                    agent: "prompt-agent".to_string(),
                    tool_mode: "normal".to_string(),
                },
                "prompt-node",
                "prompt",
            ),
            (
                PipelineNode::Prompt {
                    id: "prompt-agent-node".to_string(),
                    prompt: "Prompt text".to_string(),
                    agent: "\n\t".to_string(),
                    tool_mode: "normal".to_string(),
                },
                "prompt-agent-node",
                "agent",
            ),
            (
                PipelineNode::SequentialThinking {
                    id: "thinking-topic-node".to_string(),
                    topic: "   ".to_string(),
                    total_thoughts: 2,
                    analysis_type: None,
                    agent: "thinking-agent".to_string(),
                },
                "thinking-topic-node",
                "topic",
            ),
            (
                PipelineNode::SequentialThinking {
                    id: "thinking-agent-node".to_string(),
                    topic: "Topic".to_string(),
                    total_thoughts: 2,
                    analysis_type: None,
                    agent: "   ".to_string(),
                },
                "thinking-agent-node",
                "agent",
            ),
            (
                PipelineNode::SequentialThinking {
                    id: "thinking-count-node".to_string(),
                    topic: "Topic".to_string(),
                    total_thoughts: 0,
                    analysis_type: None,
                    agent: "thinking-agent".to_string(),
                },
                "thinking-count-node",
                "totalThoughts",
            ),
        ];

        for (node, node_id, field) in cases {
            let definition = PipelineDefinition {
                nodes: vec![node],
                edges: vec![],
            };

            let error = validate_pipeline(&definition).expect_err("invalid node should fail");
            let message = error.to_string();
            assert!(
                message.contains(node_id),
                "expected error to mention node ID '{node_id}', got: {message}"
            );
            assert!(
                message.contains(field),
                "expected error to mention field '{field}', got: {message}"
            );
        }
    }

    #[test]
    fn build_graph_preserves_node_ids_and_edge_types() {
        let definition = sample_definition();
        let graph = definition.build_graph();
        let mut nodes = graph
            .node_indices()
            .map(|node_index| graph[node_index].clone())
            .collect::<Vec<_>>();
        nodes.sort();

        let mut edges = graph
            .edge_indices()
            .map(|edge_index| {
                let (source_index, target_index) = graph
                    .edge_endpoints(edge_index)
                    .expect("edge endpoints should exist");
                (
                    graph[source_index].clone(),
                    graph[target_index].clone(),
                    graph[edge_index].clone(),
                )
            })
            .collect::<Vec<_>>();
        edges.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));

        assert_eq!(
            nodes,
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
        assert_eq!(
            edges,
            vec![
                ("a".to_string(), "b".to_string(), EdgeType::Success),
                ("b".to_string(), "c".to_string(), EdgeType::Always),
            ]
        );
    }

    #[test]
    fn edge_type_deserializes_default_to_success() {
        let edge: PipelineEdge = serde_json::from_value(json!({
            "source": "a",
            "target": "b"
        }))
        .expect("edge should deserialize");

        assert!(matches!(edge.edge_type, EdgeType::Success));
    }

    #[test]
    fn edge_type_deserializes_always_variant() {
        let edge: PipelineEdge = serde_json::from_value(json!({
            "source": "a",
            "target": "b",
            "type": "always"
        }))
        .expect("edge should deserialize");

        assert!(matches!(edge.edge_type, EdgeType::Always));
    }
}
