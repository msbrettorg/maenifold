use serde_json::json;

use super::validation_test_support::{mixed_definition, sample_definition};
use super::{EdgeType, PipelineDefinition, PipelineEdge, PipelineNode, validate_pipeline};

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
                acp_client: Some("agent-dup-a".to_string()),
                prompt: None,

                timeout: None,
            },
            PipelineNode::Workflow {
                id: "dup".to_string(),
                workflow_id: "workflow-b".to_string(),
                acp_client: Some("agent-dup-b".to_string()),
                prompt: None,

                timeout: None,
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
        mapping: None,
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
        mapping: None,
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
                acp_client: Some("agent-a".to_string()),
                prompt: None,

                timeout: None,
            },
            PipelineNode::Workflow {
                id: "b".to_string(),
                workflow_id: "workflow-b".to_string(),
                acp_client: Some("agent-b".to_string()),
                prompt: None,

                timeout: None,
            },
        ],
        edges: vec![
            PipelineEdge {
                source: "a".to_string(),
                target: "b".to_string(),
                edge_type: EdgeType::Success,
                mapping: None,
            },
            PipelineEdge {
                source: "b".to_string(),
                target: "a".to_string(),
                edge_type: EdgeType::Always,
                mapping: None,
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
                acp_client: Some("agent-a".to_string()),
                prompt: None,

                timeout: None,
            },
            PipelineNode::Workflow {
                id: "b".to_string(),
                workflow_id: "workflow-b".to_string(),
                acp_client: Some("agent-b".to_string()),
                prompt: None,

                timeout: None,
            },
        ],
        edges: vec![
            PipelineEdge {
                source: "a".to_string(),
                target: "b".to_string(),
                edge_type: EdgeType::Success,
                mapping: None,
            },
            PipelineEdge {
                source: "a".to_string(),
                target: "b".to_string(),
                edge_type: EdgeType::Always,
                mapping: None,
            },
        ],
    };

    let error = validate_pipeline(&definition).expect_err("expected duplicate edge pair to fail");
    assert!(error.to_string().contains("duplicate edge 'a -> b'"));
}

#[test]
fn validate_pipeline_accepts_mixed_edge_types_when_acyclic() {
    validate_pipeline(&sample_definition()).expect("valid pipeline should pass validation");
}

#[test]
fn validate_pipeline_accepts_all_supported_node_types() {
    validate_pipeline(&mixed_definition()).expect("mixed valid node types should pass validation");
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
