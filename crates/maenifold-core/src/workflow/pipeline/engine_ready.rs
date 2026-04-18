use super::mapping_resolve::conditional_predicate_satisfied;
use super::{EdgeType, NodeState, NodeStatus, PipelineDefinition};
use petgraph::{Direction::Incoming, prelude::NodeIndex};
use std::collections::HashMap;

pub fn compute_ready_set(
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
) -> Vec<String> {
    let graph = definition.build_graph();
    let node_indices = graph
        .node_indices()
        .map(|node_index| (graph[node_index].clone(), node_index))
        .collect::<HashMap<_, _>>();

    definition
        .nodes
        .iter()
        .filter(|node| {
            matches!(
                node_states.get(node.id()),
                Some(NodeState {
                    status: NodeStatus::Pending,
                    ..
                })
            )
        })
        .filter(|node| {
            let Some(&node_index) = node_indices.get(node.id()) else {
                return false;
            };

            !is_node_blocked(&graph, node_index, node_states)
                && incoming_edges_satisfy_ready_rule(&graph, node_index, node_states)
        })
        .map(|node| node.id().to_string())
        .collect()
}

pub fn has_remaining_work(node_states: &HashMap<String, NodeState>) -> bool {
    node_states.values().any(|state| {
        matches!(
            state.status,
            NodeStatus::Pending | NodeStatus::Ready | NodeStatus::Active | NodeStatus::Retrying
        )
    })
}

fn is_node_blocked(
    graph: &petgraph::stable_graph::StableGraph<String, EdgeType>,
    node_index: NodeIndex,
    node_states: &HashMap<String, NodeState>,
) -> bool {
    graph
        .neighbors_directed(node_index, Incoming)
        .any(|source_index| {
            let source_id = graph[source_index].as_str();
            graph
                .edges_connecting(source_index, node_index)
                .any(|edge| {
                    matches!(
                        edge.weight(),
                        EdgeType::Success
                    ) && matches!(
                        node_states.get(source_id).map(|state| &state.status),
                        Some(NodeStatus::Abandoned | NodeStatus::Stuck)
                    )
                })
        })
}

fn incoming_edges_satisfy_ready_rule(
    graph: &petgraph::stable_graph::StableGraph<String, EdgeType>,
    node_index: NodeIndex,
    node_states: &HashMap<String, NodeState>,
) -> bool {
    let mut has_incoming = false;
    let mut has_satisfied_edge = false;
    let mut has_non_success_edge = false;
    let mut has_satisfying_non_success_edge = false;

    for source_index in graph.neighbors_directed(node_index, Incoming) {
        let source_id = graph[source_index].as_str();
        for edge in graph.edges_connecting(source_index, node_index) {
            has_incoming = true;
            let satisfied = is_edge_satisfied(edge.weight(), source_id, node_states);
            match edge.weight() {
                EdgeType::Success => {
                    if !satisfied {
                        return false;
                    }
                    has_satisfied_edge = true;
                }
                EdgeType::Always | EdgeType::Conditional { .. } => {
                    has_non_success_edge = true;
                    if satisfied {
                        has_satisfied_edge = true;
                        has_satisfying_non_success_edge = true;
                    }
                }
            }
        }
    }

    if !has_incoming {
        return true;
    }
    if !has_satisfied_edge {
        return false;
    }
    if has_non_success_edge {
        has_satisfying_non_success_edge
    } else {
        true
    }
}

fn is_edge_satisfied(
    edge_type: &EdgeType,
    source_id: &str,
    node_states: &HashMap<String, NodeState>,
) -> bool {
    match (edge_type, node_states.get(source_id)) {
        (EdgeType::Success, Some(state)) => matches!(state.status, NodeStatus::Completed),
        (EdgeType::Always, Some(state)) => {
            matches!(
                state.status,
                NodeStatus::Completed | NodeStatus::Abandoned | NodeStatus::Stuck
            )
        }
        (EdgeType::Conditional { path, equals }, Some(state)) => {
            matches!(state.status, NodeStatus::Completed)
                && conditional_predicate_satisfied(state.output.as_deref(), path, equals)
        }
        (_, None) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::pipeline::types::{PipelineEdge, PipelineNode};

    fn completed_state(output: &str) -> NodeState {
        NodeState {
            session_id: "s".into(),
            status: NodeStatus::Completed,
            attempts: 1,
            acp_session_id: None,
            thinking_session_id: None,
            tool_provenance: None,
            output: Some(output.into()),
            failure_reason: None,
            retry_after: None,
            retry_count: None,
        }
    }

    fn pending_state() -> NodeState {
        NodeState {
            session_id: "s".into(),
            status: NodeStatus::Pending,
            attempts: 0,
            acp_session_id: None,
            thinking_session_id: None,
            tool_provenance: None,
            output: None,
            failure_reason: None,
            retry_after: None,
            retry_count: None,
        }
    }

    fn make_thinking_node(id: &str) -> PipelineNode {
        PipelineNode::SequentialThinking {
            id: id.into(),
            agent: "agent".into(),
            topic: "t".into(),
            total_thoughts: 1,
            thought_level: None,
            allowed_tools: None,
            max_turns: None,
            cwd: None,
            display_label: None,
            timeout: None,
            agent_config: None,
            model: None,
            analysis_type: None,
        }
    }

    #[test]
    fn conditional_edge_satisfied_when_path_matches() {
        let mut states = HashMap::new();
        states.insert("up".to_string(), completed_state(r#"{"verdict":"FAIL"}"#));
        let edge = EdgeType::Conditional {
            path: "verdict".into(),
            equals: serde_json::Value::String("FAIL".into()),
        };
        assert!(is_edge_satisfied(&edge, "up", &states));
    }

    #[test]
    fn conditional_edge_not_satisfied_when_value_differs() {
        let mut states = HashMap::new();
        states.insert("up".to_string(), completed_state(r#"{"verdict":"PASS"}"#));
        let edge = EdgeType::Conditional {
            path: "verdict".into(),
            equals: serde_json::Value::String("FAIL".into()),
        };
        assert!(!is_edge_satisfied(&edge, "up", &states));
    }

    #[test]
    fn conditional_edge_not_satisfied_when_path_missing() {
        let mut states = HashMap::new();
        states.insert("up".to_string(), completed_state(r#"{"other":"FAIL"}"#));
        let edge = EdgeType::Conditional {
            path: "verdict".into(),
            equals: serde_json::Value::String("FAIL".into()),
        };
        assert!(!is_edge_satisfied(&edge, "up", &states));
    }

    #[test]
    fn conditional_edge_not_satisfied_when_output_not_json() {
        let mut states = HashMap::new();
        states.insert("up".to_string(), completed_state("plain text"));
        let edge = EdgeType::Conditional {
            path: "verdict".into(),
            equals: serde_json::Value::String("FAIL".into()),
        };
        assert!(!is_edge_satisfied(&edge, "up", &states));
    }

    #[test]
    fn conditional_edge_supports_number_and_bool_equality() {
        let mut states = HashMap::new();
        states.insert(
            "up".to_string(),
            completed_state(r#"{"score":42,"ok":true}"#),
        );
        let num_edge = EdgeType::Conditional {
            path: "score".into(),
            equals: serde_json::json!(42),
        };
        let bool_edge = EdgeType::Conditional {
            path: "ok".into(),
            equals: serde_json::json!(true),
        };
        assert!(is_edge_satisfied(&num_edge, "up", &states));
        assert!(is_edge_satisfied(&bool_edge, "up", &states));
    }

    #[test]
    fn conditional_only_target_with_unmet_predicate_is_not_ready() {
        let mut states = HashMap::new();
        states.insert("up".to_string(), completed_state(r#"{"verdict":"PASS"}"#));
        states.insert("down".to_string(), pending_state());
        let definition = PipelineDefinition {
            nodes: vec![make_thinking_node("up"), make_thinking_node("down")],
            edges: vec![PipelineEdge {
                source: "up".into(),
                target: "down".into(),
                edge_type: EdgeType::Conditional {
                    path: "verdict".into(),
                    equals: serde_json::Value::String("FAIL".into()),
                },
                mapping: None,
            }],
            template_id: None,
            inputs: None,
        };
        let ready = compute_ready_set(&definition, &states);
        assert!(
            ready.is_empty(),
            "expected no ready nodes, got: {:?}",
            ready
        );
    }

    #[test]
    fn conditional_target_becomes_ready_when_predicate_matches() {
        let mut states = HashMap::new();
        states.insert("up".to_string(), completed_state(r#"{"verdict":"FAIL"}"#));
        states.insert("down".to_string(), pending_state());
        let definition = PipelineDefinition {
            nodes: vec![make_thinking_node("up"), make_thinking_node("down")],
            edges: vec![PipelineEdge {
                source: "up".into(),
                target: "down".into(),
                edge_type: EdgeType::Conditional {
                    path: "verdict".into(),
                    equals: serde_json::Value::String("FAIL".into()),
                },
                mapping: None,
            }],
            template_id: None,
            inputs: None,
        };
        let ready = compute_ready_set(&definition, &states);
        assert_eq!(ready, vec!["down".to_string()]);
    }

    #[test]
    fn always_edge_can_satisfy_target_when_conditional_is_false() {
        let mut states = HashMap::new();
        states.insert("cond".to_string(), completed_state(r#"{"verdict":"PASS"}"#));
        states.insert("always".to_string(), completed_state(r#"{"ok":true}"#));
        states.insert("down".to_string(), pending_state());
        let definition = PipelineDefinition {
            nodes: vec![
                make_thinking_node("cond"),
                make_thinking_node("always"),
                make_thinking_node("down"),
            ],
            edges: vec![
                PipelineEdge {
                    source: "cond".into(),
                    target: "down".into(),
                    edge_type: EdgeType::Conditional {
                        path: "verdict".into(),
                        equals: serde_json::Value::String("FAIL".into()),
                    },
                    mapping: None,
                },
                PipelineEdge {
                    source: "always".into(),
                    target: "down".into(),
                    edge_type: EdgeType::Always,
                    mapping: None,
                },
            ],
            template_id: None,
            inputs: None,
        };

        let ready = compute_ready_set(&definition, &states);
        assert_eq!(ready, vec!["down".to_string()]);
    }
}
