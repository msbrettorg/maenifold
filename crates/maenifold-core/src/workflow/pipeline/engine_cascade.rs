use super::{EdgeType, JoinPolicy, NodeState, NodeStatus, PipelineDefinition};
use petgraph::{
    Direction::{Incoming, Outgoing},
    prelude::NodeIndex,
    visit::EdgeRef,
};
use std::collections::{HashMap, HashSet, VecDeque};

pub fn compute_blocked_cascade(
    definition: &PipelineDefinition,
    node_states: &HashMap<String, NodeState>,
) -> Vec<String> {
    let graph = definition.build_graph();
    let join_policies = definition
        .nodes
        .iter()
        .map(|node| (node.id().to_string(), node.join_policy()))
        .collect::<HashMap<_, _>>();
    let mut blocked_queue = graph
        .node_indices()
        .filter(|&node_index| {
            matches!(
                node_states.get(&graph[node_index]),
                Some(NodeState {
                    status: NodeStatus::Abandoned | NodeStatus::Stuck | NodeStatus::Failed,
                    ..
                })
            )
        })
        .collect::<VecDeque<_>>();
    let mut cascaded = Vec::new();
    let mut cascaded_set = HashSet::new();

    while let Some(source_index) = blocked_queue.pop_front() {
        for edge in graph.edges_directed(source_index, Outgoing) {
            if !matches!(
                edge.weight(),
                EdgeType::Success | EdgeType::Conditional { .. }
            ) {
                continue;
            }

            let target_index = edge.target();
            let target_id = graph[target_index].clone();
            if cascaded_set.contains(target_id.as_str())
                || !is_cascade_eligible(
                    &graph,
                    target_index,
                    node_states,
                    &cascaded_set,
                    &join_policies,
                )
            {
                continue;
            }

            cascaded_set.insert(target_id.clone());
            cascaded.push(target_id);
            blocked_queue.push_back(target_index);
        }
    }

    cascaded
}
fn is_cascade_eligible(
    graph: &petgraph::stable_graph::StableGraph<String, EdgeType>,
    node_index: NodeIndex,
    node_states: &HashMap<String, NodeState>,
    cascaded: &HashSet<String>,
    join_policies: &HashMap<String, JoinPolicy>,
) -> bool {
    let node_id = graph[node_index].as_str();
    if !matches!(
        effective_node_status(node_id, node_states, cascaded),
        Some(NodeStatus::Pending | NodeStatus::Ready | NodeStatus::Retrying)
    ) {
        return false;
    }
    let join_policy = join_policies.get(node_id).unwrap_or(&JoinPolicy::All);
    if !matches!(join_policy, JoinPolicy::All) {
        return is_any_or_first_cascade_eligible(graph, node_index, node_states, cascaded);
    }

    let mut has_incoming_requirement = false;
    for source_index in graph.neighbors_directed(node_index, Incoming) {
        let source_id = graph[source_index].as_str();
        for edge in graph.edges_connecting(source_index, node_index) {
            match edge.weight() {
                EdgeType::Always => {
                    if !matches!(
                        effective_node_status(source_id, node_states, cascaded),
                        Some(NodeStatus::Abandoned | NodeStatus::Stuck | NodeStatus::Failed)
                    ) {
                        return false;
                    }
                }
                EdgeType::Success | EdgeType::Conditional { .. } => {
                    has_incoming_requirement = true;
                    if !matches!(
                        effective_node_status(source_id, node_states, cascaded),
                        Some(NodeStatus::Abandoned | NodeStatus::Stuck | NodeStatus::Failed)
                    ) {
                        return false;
                    }
                }
            }
        }
    }
    has_incoming_requirement
}
fn is_any_or_first_cascade_eligible(
    graph: &petgraph::stable_graph::StableGraph<String, EdgeType>,
    node_index: NodeIndex,
    node_states: &HashMap<String, NodeState>,
    cascaded: &HashSet<String>,
) -> bool {
    let mut has_incoming_requirement = false;
    let mut has_firing_predecessor = false;
    let mut all_requirements_resolved = true;

    for source_index in graph.neighbors_directed(node_index, Incoming) {
        let source_id = graph[source_index].as_str();
        for edge in graph.edges_connecting(source_index, node_index) {
            match edge.weight() {
                EdgeType::Always => {
                    if !matches!(
                        effective_node_status(source_id, node_states, cascaded),
                        Some(NodeStatus::Abandoned | NodeStatus::Stuck | NodeStatus::Failed)
                    ) {
                        return false;
                    }
                }
                EdgeType::Success => {
                    has_incoming_requirement = true;
                    match effective_node_status(source_id, node_states, cascaded) {
                        Some(NodeStatus::Completed) => has_firing_predecessor = true,
                        Some(
                            NodeStatus::Skipped
                            | NodeStatus::Abandoned
                            | NodeStatus::Stuck
                            | NodeStatus::Failed,
                        ) => {}
                        _ => all_requirements_resolved = false,
                    }
                }
                EdgeType::Conditional { .. } => {
                    has_incoming_requirement = true;
                    if super::engine_ready::is_edge_satisfied(edge.weight(), source_id, node_states)
                    {
                        has_firing_predecessor = true;
                    } else if !matches!(
                        effective_node_status(source_id, node_states, cascaded),
                        Some(
                            NodeStatus::Completed
                                | NodeStatus::Skipped
                                | NodeStatus::Abandoned
                                | NodeStatus::Stuck,
                        )
                    ) {
                        all_requirements_resolved = false;
                    }
                }
            }
        }
    }
    has_incoming_requirement && !has_firing_predecessor && all_requirements_resolved
}
fn effective_node_status<'a>(
    node_id: &'a str,
    node_states: &'a HashMap<String, NodeState>,
    cascaded: &HashSet<String>,
) -> Option<NodeStatus> {
    if cascaded.contains(node_id) {
        Some(NodeStatus::Abandoned)
    } else {
        node_states.get(node_id).map(|state| state.status.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::pipeline::types::{PipelineEdge, PipelineNode};

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
            adopt: None,
            requires_role: None,
            model: None,
            retry_policy: None,
            join_policy: None,
            output_schema: None,
        }
    }

    fn node_state(status: NodeStatus) -> NodeState {
        NodeState {
            session_id: "s".into(),
            status,
            attempts: 1,
            started_at: None,
            completed_at: None,
            acp_session_id: None,
            thinking_session_id: None,
            tool_provenance: None,
            input_tokens: None,
            output_tokens: None,
            model: None,
            output: None,
            output_artifact: None,
            failure_reason: None,
            retry_after: None,
            retry_count: None,
        }
    }

    #[test]
    fn cascade_skips_target_with_non_abandoned_always_fallback() {
        let definition = PipelineDefinition {
            nodes: vec![
                make_thinking_node("cond"),
                make_thinking_node("always"),
                make_thinking_node("target"),
            ],
            edges: vec![
                PipelineEdge {
                    source: "cond".into(),
                    target: "target".into(),
                    edge_type: EdgeType::Conditional {
                        path: "verdict".into(),
                        equals: serde_json::Value::String("FAIL".into()),
                    },
                    mode: Default::default(),
                    mapping: None,
                },
                PipelineEdge {
                    source: "always".into(),
                    target: "target".into(),
                    edge_type: EdgeType::Always,
                    mode: Default::default(),
                    mapping: None,
                },
            ],
            template_id: None,
            inputs: None,
            input_schema: None,
        };
        let node_states = HashMap::from([
            ("cond".to_string(), node_state(NodeStatus::Abandoned)),
            ("always".to_string(), node_state(NodeStatus::Pending)),
            ("target".to_string(), node_state(NodeStatus::Pending)),
        ]);

        let cascaded = compute_blocked_cascade(&definition, &node_states);
        assert!(
            cascaded.is_empty(),
            "expected no cascade with Always fallback, got: {cascaded:?}"
        );
    }

    #[test]
    fn cascade_propagates_from_skipped_conditional_node_to_descendants() {
        let definition = PipelineDefinition {
            nodes: vec![
                make_thinking_node("branch"),
                make_thinking_node("child"),
                make_thinking_node("grandchild"),
            ],
            edges: vec![
                PipelineEdge {
                    source: "branch".into(),
                    target: "child".into(),
                    edge_type: EdgeType::Success,
                    mode: Default::default(),
                    mapping: None,
                },
                PipelineEdge {
                    source: "child".into(),
                    target: "grandchild".into(),
                    edge_type: EdgeType::Success,
                    mode: Default::default(),
                    mapping: None,
                },
            ],
            template_id: None,
            inputs: None,
            input_schema: None,
        };
        let node_states = HashMap::from([
            ("branch".to_string(), node_state(NodeStatus::Abandoned)),
            ("child".to_string(), node_state(NodeStatus::Pending)),
            ("grandchild".to_string(), node_state(NodeStatus::Pending)),
        ]);

        let cascaded = compute_blocked_cascade(&definition, &node_states);
        assert_eq!(
            cascaded,
            vec!["child".to_string(), "grandchild".to_string()]
        );
    }
}
