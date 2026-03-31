//! Engine-level smoke tests — tests pure functions (compute_ready_set, validate_pipeline,
//! parse_edge, mapped_tool_args) directly. These intentionally bypass the handler harness
//! because they test engine logic independent of I/O.

use super::engine::compute_ready_set;
use super::executor::predecessor_context_content;
use super::executor_mapping::mapped_tool_args;
use super::executor_tests_support::{
    completed_node_state, pending_node_state, pipeline_test_context, source_tool_node,
    target_tool_node,
};
use super::{
    EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode,
    validate_pipeline,
};
use crate::util::json::OutputFormat;
use serde_json::{Value, json};
use serial_test::serial;
use std::collections::HashMap;

fn wf(id: &str) -> PipelineNode {
    PipelineNode::Workflow {
        id: id.into(),
        workflow_id: format!("workflow-{id}"),
        acp_client: None,
        prompt: None,
        timeout: None,
    }
}
fn edge(source: &str, target: &str, edge_type: EdgeType) -> PipelineEdge {
    PipelineEdge {
        source: source.into(),
        target: target.into(),
        edge_type,
        mapping: None,
    }
}
fn def(nodes: Vec<PipelineNode>, edges: Vec<PipelineEdge>) -> PipelineDefinition {
    PipelineDefinition { nodes, edges }
}
fn state(id: &str, status: NodeStatus) -> (String, NodeState) {
    (
        id.into(),
        NodeState {
            session_id: format!("session-{id}"),
            status,
            attempts: 0,
            thinking_session_id: None,
            output: None,
        },
    )
}
fn parse_edge(value: Value) -> Result<PipelineEdge, serde_json::Error> {
    serde_json::from_value(value)
}

#[test]
#[serial(config)]
fn e_01_success_edge_blocks_on_abandoned_predecessor() {
    let d = def(
        vec![wf("a"), wf("b")],
        vec![edge("a", "b", EdgeType::Success)],
    );
    assert!(
        compute_ready_set(
            &d,
            &HashMap::from([
                state("a", NodeStatus::Abandoned),
                state("b", NodeStatus::Pending)
            ])
        )
        .is_empty()
    );
}
#[test]
#[serial(config)]
fn e_02_success_edge_blocks_on_stuck_predecessor() {
    let d = def(
        vec![wf("a"), wf("b")],
        vec![edge("a", "b", EdgeType::Success)],
    );
    assert!(
        compute_ready_set(
            &d,
            &HashMap::from([
                state("a", NodeStatus::Stuck),
                state("b", NodeStatus::Pending)
            ])
        )
        .is_empty()
    );
}
#[test]
#[serial(config)]
fn e_03_success_edge_activates_on_completed_predecessor() {
    let d = def(
        vec![wf("a"), wf("b")],
        vec![edge("a", "b", EdgeType::Success)],
    );
    assert_eq!(
        compute_ready_set(
            &d,
            &HashMap::from([
                state("a", NodeStatus::Completed),
                state("b", NodeStatus::Pending)
            ])
        ),
        vec!["b".to_string()]
    );
}
#[test]
#[serial(config)]
fn e_04_always_edge_activates_on_completed_predecessor() {
    let d = def(
        vec![wf("a"), wf("b")],
        vec![edge("a", "b", EdgeType::Always)],
    );
    assert_eq!(
        compute_ready_set(
            &d,
            &HashMap::from([
                state("a", NodeStatus::Completed),
                state("b", NodeStatus::Pending)
            ])
        ),
        vec!["b".to_string()]
    );
}
#[test]
#[serial(config)]
fn e_05_always_edge_activates_on_abandoned_predecessor() {
    let d = def(
        vec![wf("a"), wf("b")],
        vec![edge("a", "b", EdgeType::Always)],
    );
    assert_eq!(
        compute_ready_set(
            &d,
            &HashMap::from([
                state("a", NodeStatus::Abandoned),
                state("b", NodeStatus::Pending)
            ])
        ),
        vec!["b".to_string()]
    );
}
#[test]
#[serial(config)]
fn e_06_always_edge_activates_on_stuck_predecessor() {
    let d = def(
        vec![wf("a"), wf("b")],
        vec![edge("a", "b", EdgeType::Always)],
    );
    assert_eq!(
        compute_ready_set(
            &d,
            &HashMap::from([
                state("a", NodeStatus::Stuck),
                state("b", NodeStatus::Pending)
            ])
        ),
        vec!["b".to_string()]
    );
}
#[test]
#[serial(config)]
fn e_07_default_edge_type_is_success() {
    let d = def(
        vec![wf("a"), wf("b")],
        vec![parse_edge(json!({"source":"a","target":"b"})).expect("edge")],
    );
    assert!(
        compute_ready_set(
            &d,
            &HashMap::from([
                state("a", NodeStatus::Active),
                state("b", NodeStatus::Pending)
            ])
        )
        .is_empty()
    );
}
#[test]
#[serial(config)]
fn e_08_node_with_two_success_deps_waits_for_both() {
    let d = def(
        vec![wf("a"), wf("b"), wf("c")],
        vec![
            edge("a", "c", EdgeType::Success),
            edge("b", "c", EdgeType::Success),
        ],
    );
    assert_eq!(
        compute_ready_set(
            &d,
            &HashMap::from([
                state("a", NodeStatus::Completed),
                state("b", NodeStatus::Pending),
                state("c", NodeStatus::Pending)
            ])
        ),
        vec!["b".to_string()]
    );
    assert_eq!(
        compute_ready_set(
            &d,
            &HashMap::from([
                state("a", NodeStatus::Completed),
                state("b", NodeStatus::Completed),
                state("c", NodeStatus::Pending)
            ])
        ),
        vec!["c".to_string()]
    );
}
#[test]
#[serial(config)]
fn e_09_node_with_success_and_always_deps_uses_both_rules() {
    let d = def(
        vec![wf("a"), wf("b"), wf("c")],
        vec![
            edge("a", "c", EdgeType::Success),
            edge("b", "c", EdgeType::Always),
        ],
    );
    assert_eq!(
        compute_ready_set(
            &d,
            &HashMap::from([
                state("a", NodeStatus::Completed),
                state("b", NodeStatus::Abandoned),
                state("c", NodeStatus::Pending)
            ])
        ),
        vec!["c".to_string()]
    );
}
#[test]
#[serial(config)]
fn ef_01_edge_type_success_explicit() {
    assert_eq!(
        parse_edge(json!({"source":"a","target":"b","type":"success"}))
            .expect("edge")
            .edge_type,
        EdgeType::Success
    );
}
#[test]
#[serial(config)]
fn ef_02_edge_type_always_explicit() {
    assert_eq!(
        parse_edge(json!({"source":"a","target":"b","type":"always"}))
            .expect("edge")
            .edge_type,
        EdgeType::Always
    );
}
#[test]
#[serial(config)]
fn ef_03_edge_type_absent_defaults_to_success() {
    assert_eq!(
        parse_edge(json!({"source":"a","target":"b"}))
            .expect("edge")
            .edge_type,
        EdgeType::Success
    );
}
#[test]
#[serial(config)]
fn ef_04_edge_type_invalid_value_rejected() {
    assert!(parse_edge(json!({"source":"a","target":"b","type":"nope"})).is_err());
}
#[test]
#[serial(config)]
fn ef_05_edge_type_null_rejected() {
    assert!(parse_edge(json!({"source":"a","target":"b","type":null})).is_err());
}
#[test]
#[serial(config)]
fn ef_06_mapping_valid_field_mapping() {
    let args = mapped_tool_args(
        &def(
            vec![source_tool_node("source"), target_tool_node(None)],
            vec![PipelineEdge {
                source: "source".into(),
                target: "tool".into(),
                edge_type: EdgeType::Success,
                mapping: Some(HashMap::from([("title".into(), "data.title".into())])),
            }],
        ),
        &HashMap::from([
            completed_node_state("source", "ef-06-source", json!({"data":{"title":"ok"}})),
            pending_node_state("tool", "ef-06-tool"),
        ]),
        "tool",
        None,
    );
    assert_eq!(args, Some(json!({"title":"ok"})));
}
#[test]
#[serial(config)]
fn ef_07_mapping_empty_object() {
    let edge = parse_edge(json!({"source":"a","target":"b","mapping":{}})).expect("edge");
    assert_eq!(edge.mapping, Some(HashMap::new()));
}
#[test]
#[serial(config)]
fn ef_08_mapping_absent() {
    assert_eq!(
        parse_edge(json!({"source":"a","target":"b"}))
            .expect("edge")
            .mapping,
        None
    );
}
#[test]
#[serial(config)]
fn ef_09_mapping_nested_path_resolution() {
    let args = mapped_tool_args(
        &def(
            vec![source_tool_node("source"), target_tool_node(None)],
            vec![PipelineEdge {
                source: "source".into(),
                target: "tool".into(),
                edge_type: EdgeType::Success,
                mapping: Some(HashMap::from([("body".into(), "data.content".into())])),
            }],
        ),
        &HashMap::from([
            completed_node_state(
                "source",
                "ef-09-source",
                json!({"data":{"content":"nested"}}),
            ),
            pending_node_state("tool", "ef-09-tool"),
        ]),
        "tool",
        None,
    );
    assert_eq!(args, Some(json!({"body":"nested"})));
}
#[test]
#[serial(config)]
fn ef_10_mapping_array_index_resolution() {
    let args = mapped_tool_args(
        &def(
            vec![source_tool_node("source"), target_tool_node(None)],
            vec![PipelineEdge {
                source: "source".into(),
                target: "tool".into(),
                edge_type: EdgeType::Success,
                mapping: Some(HashMap::from([("title".into(), "results[0].title".into())])),
            }],
        ),
        &HashMap::from([
            completed_node_state(
                "source",
                "ef-10-source",
                json!({"results":[{"title":"first"}]}),
            ),
            pending_node_state("tool", "ef-10-tool"),
        ]),
        "tool",
        None,
    );
    assert_eq!(args, Some(json!({"title":"first"})));
}
#[test]
#[serial(config)]
fn ef_11_source_empty_string_rejected() {
    assert!(
        validate_pipeline(&def(
            vec![wf("a"), wf("b")],
            vec![parse_edge(json!({"source":"","target":"b"})).expect("edge")]
        ))
        .expect_err("err")
        .to_string()
        .contains("unknown source node")
    );
}
#[test]
#[serial(config)]
fn ef_12_target_empty_string_rejected() {
    assert!(
        validate_pipeline(&def(
            vec![wf("a"), wf("b")],
            vec![parse_edge(json!({"source":"a","target":""})).expect("edge")]
        ))
        .expect_err("err")
        .to_string()
        .contains("unknown target node")
    );
}
#[test]
#[serial(config)]
fn ef_13_source_equals_target_rejected() {
    assert!(
        validate_pipeline(&def(
            vec![wf("a")],
            vec![parse_edge(json!({"source":"a","target":"a"})).expect("edge")]
        ))
        .expect_err("err")
        .to_string()
        .contains("cycle")
    );
}
#[test]
#[serial(config)]
fn pc_01_completed_predecessor_output_available_to_successor() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    let content = predecessor_context_content(
        &c,
        &def(
            vec![wf("a"), wf("b")],
            vec![edge("a", "b", EdgeType::Success)],
        ),
        &HashMap::from([
            (
                "a".into(),
                NodeState {
                    session_id: "pc-01-a".into(),
                    status: NodeStatus::Completed,
                    attempts: 0,
                    thinking_session_id: None,
                    output: Some("upstream".into()),
                },
            ),
            state("b", NodeStatus::Pending),
        ]),
        "b",
    )
    .expect("context");
    assert_eq!(content, Some("### From a\n\nupstream".into()));
}
#[test]
#[serial(config)]
fn pc_02_multiple_predecessors_all_contribute_context() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    let content = predecessor_context_content(
        &c,
        &def(
            vec![wf("a"), wf("b"), wf("c")],
            vec![
                edge("a", "c", EdgeType::Success),
                edge("b", "c", EdgeType::Always),
            ],
        ),
        &HashMap::from([
            (
                "a".into(),
                NodeState {
                    session_id: "pc-02-a".into(),
                    status: NodeStatus::Completed,
                    attempts: 0,
                    thinking_session_id: None,
                    output: Some("one".into()),
                },
            ),
            (
                "b".into(),
                NodeState {
                    session_id: "pc-02-b".into(),
                    status: NodeStatus::Completed,
                    attempts: 0,
                    thinking_session_id: None,
                    output: Some("two".into()),
                },
            ),
            state("c", NodeStatus::Pending),
        ]),
        "c",
    )
    .expect("context")
    .expect("some");
    assert!(content.contains("### From a\n\none"));
    assert!(content.contains("### From b\n\ntwo"));
}
#[test]
#[serial(config)]
fn pc_03_abandoned_predecessor_has_no_output_in_context() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    let content = predecessor_context_content(
        &c,
        &def(
            vec![wf("a"), wf("b")],
            vec![edge("a", "b", EdgeType::Always)],
        ),
        &HashMap::from([
            state("a", NodeStatus::Abandoned),
            state("b", NodeStatus::Pending),
        ]),
        "b",
    )
    .expect("context");
    assert_eq!(content, None);
}
#[test]
#[serial(config)]
fn pc_04_tool_node_edge_mapping_resolves_predecessor_output() {
    let args = mapped_tool_args(
        &def(
            vec![source_tool_node("source"), target_tool_node(None)],
            vec![PipelineEdge {
                source: "source".into(),
                target: "tool".into(),
                edge_type: EdgeType::Success,
                mapping: Some(HashMap::from([("title".into(), "results[0].title".into())])),
            }],
        ),
        &HashMap::from([
            completed_node_state(
                "source",
                "pc-04-source",
                json!({"results":[{"title":"mapped"}]}),
            ),
            pending_node_state("tool", "pc-04-tool"),
        ]),
        "tool",
        None,
    );
    assert_eq!(args, Some(json!({"title":"mapped"})));
}
#[test]
#[serial(config)]
fn pc_05_unresolved_mapping_path_becomes_null() {
    let args = mapped_tool_args(
        &def(
            vec![source_tool_node("source"), target_tool_node(None)],
            vec![PipelineEdge {
                source: "source".into(),
                target: "tool".into(),
                edge_type: EdgeType::Success,
                mapping: Some(HashMap::from([(
                    "missing".into(),
                    "results[9].title".into(),
                )])),
            }],
        ),
        &HashMap::from([
            completed_node_state(
                "source",
                "pc-05-source",
                json!({"results":[{"title":"mapped"}]}),
            ),
            pending_node_state("tool", "pc-05-tool"),
        ]),
        "tool",
        None,
    );
    assert_eq!(args, Some(json!({"missing": Value::Null})));
}
