use crate::error::{MaenifoldError, Result};
use std::collections::{HashMap, HashSet, VecDeque};

mod engine;
mod handler;
mod types;

pub use engine::{compute_pipeline_status, compute_ready_set};
pub use handler::pipeline;
pub use types::{EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode};

pub fn validate_pipeline(definition: &PipelineDefinition) -> Result<()> {
    if definition.nodes.is_empty() {
        return Err(MaenifoldError::InvalidArgument(
            "Pipeline definition must contain at least one node".to_string(),
        ));
    }

    let mut node_ids = HashSet::with_capacity(definition.nodes.len());
    let mut in_degree = HashMap::with_capacity(definition.nodes.len());
    let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::with_capacity(definition.nodes.len());

    for node in &definition.nodes {
        if !node_ids.insert(node.id.as_str()) {
            return Err(MaenifoldError::InvalidArgument(format!(
                "Pipeline contains duplicate node ID '{}'",
                node.id
            )));
        }

        in_degree.insert(node.id.as_str(), 0usize);
        adjacency.entry(node.id.as_str()).or_default();
    }

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

        adjacency
            .entry(edge.source.as_str())
            .or_default()
            .push(edge.target.as_str());
        *in_degree.entry(edge.target.as_str()).or_default() += 1;
    }

    let mut ready = VecDeque::from_iter(definition.nodes.iter().filter_map(|node| {
        (in_degree.get(node.id.as_str()) == Some(&0)).then_some(node.id.as_str())
    }));
    let mut processed = 0usize;

    while let Some(node_id) = ready.pop_front() {
        processed += 1;

        if let Some(targets) = adjacency.get(node_id) {
            for target in targets {
                if let Some(target_degree) = in_degree.get_mut(target) {
                    *target_degree -= 1;
                    if *target_degree == 0 {
                        ready.push_back(target);
                    }
                }
            }
        }
    }

    if processed != definition.nodes.len() {
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
                PipelineNode {
                    id: "a".to_string(),
                    workflow_id: "workflow-a".to_string(),
                    acp_client: None,
                },
                PipelineNode {
                    id: "b".to_string(),
                    workflow_id: "workflow-b".to_string(),
                    acp_client: Some("client-b".to_string()),
                },
                PipelineNode {
                    id: "c".to_string(),
                    workflow_id: "workflow-c".to_string(),
                    acp_client: None,
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

    #[test]
    fn pipeline_validation_accepts_valid_acyclic_definition() {
        let definition = sample_definition();

        assert!(validate_pipeline(&definition).is_ok());
    }

    #[test]
    fn pipeline_validation_rejects_empty_nodes() {
        let definition = PipelineDefinition {
            nodes: Vec::new(),
            edges: Vec::new(),
        };

        let err = validate_pipeline(&definition).expect_err("empty pipeline should fail");
        assert!(
            err.to_string().contains("at least one node"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn pipeline_validation_rejects_duplicate_node_ids() {
        let definition = PipelineDefinition {
            nodes: vec![
                PipelineNode {
                    id: "duplicate".to_string(),
                    workflow_id: "workflow-a".to_string(),
                    acp_client: None,
                },
                PipelineNode {
                    id: "duplicate".to_string(),
                    workflow_id: "workflow-b".to_string(),
                    acp_client: None,
                },
            ],
            edges: Vec::new(),
        };

        let err = validate_pipeline(&definition).expect_err("duplicate nodes should fail");
        assert!(
            err.to_string().contains("duplicate node ID"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn pipeline_validation_rejects_unknown_edge_source() {
        let mut definition = sample_definition();
        definition.edges.push(PipelineEdge {
            source: "missing".to_string(),
            target: "a".to_string(),
            edge_type: EdgeType::Success,
        });

        let err = validate_pipeline(&definition).expect_err("missing source should fail");
        assert!(
            err.to_string().contains("unknown source node"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn pipeline_validation_rejects_unknown_edge_target() {
        let mut definition = sample_definition();
        definition.edges.push(PipelineEdge {
            source: "a".to_string(),
            target: "missing".to_string(),
            edge_type: EdgeType::Success,
        });

        let err = validate_pipeline(&definition).expect_err("missing target should fail");
        assert!(
            err.to_string().contains("unknown target node"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn pipeline_validation_rejects_cycles() {
        let definition = PipelineDefinition {
            nodes: vec![
                PipelineNode {
                    id: "a".to_string(),
                    workflow_id: "workflow-a".to_string(),
                    acp_client: None,
                },
                PipelineNode {
                    id: "b".to_string(),
                    workflow_id: "workflow-b".to_string(),
                    acp_client: None,
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

        let err = validate_pipeline(&definition).expect_err("cycle should fail");
        assert!(
            err.to_string().contains("contains a cycle"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn pipeline_types_deserialize_with_camel_case_and_default_edge_type() {
        let definition: PipelineDefinition = serde_json::from_value(json!({
            "nodes": [
                {
                    "id": "node-1",
                    "workflowId": "workflow-1",
                    "acpClient": "acp-a"
                },
                {
                    "id": "node-2",
                    "workflowId": "workflow-2",
                    "acpClient": null
                }
            ],
            "edges": [
                {
                    "source": "node-1",
                    "target": "node-2"
                }
            ]
        }))
        .expect("pipeline definition should deserialize");

        assert_eq!(definition.nodes[0].workflow_id, "workflow-1");
        assert_eq!(definition.nodes[0].acp_client.as_deref(), Some("acp-a"));
        assert_eq!(definition.edges[0].edge_type, EdgeType::Success);
    }

    #[test]
    fn pipeline_types_serialize_enums_and_type_field_for_json_compatibility() {
        let edge = PipelineEdge {
            source: "a".to_string(),
            target: "b".to_string(),
            edge_type: EdgeType::Always,
        };
        let state = NodeState {
            session_id: "session-1".to_string(),
            status: NodeStatus::Completed,
            attempts: 2,
        };

        let edge_json = serde_json::to_value(&edge).expect("edge serializes");
        let state_json = serde_json::to_value(&state).expect("state serializes");

        assert_eq!(edge_json["type"], "always");
        assert_eq!(edge_json["source"], "a");
        assert_eq!(edge_json["target"], "b");
        assert_eq!(state_json["sessionId"], "session-1");
        assert_eq!(state_json["status"], "completed");
        assert_eq!(state_json["attempts"], 2);
    }
}
