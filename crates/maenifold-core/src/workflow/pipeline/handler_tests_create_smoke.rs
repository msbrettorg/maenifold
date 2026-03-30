use super::handler::pipeline;
use super::test_support::*;
use super::session_io::frontmatter_value;
use super::EdgeType;
use super::PipelineDefinition;
use crate::markdown::{file_exists, read_markdown};
use crate::sessions::{pipeline_session_uri, workflow_session_uri};
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use serde_json::{json, Value};
use serde_yaml::Value as YamlValue;
use serial_test::serial;

fn payload(nodes: Vec<Value>, edges: Vec<Value>) -> Value {
    json!({ "definition": { "nodes": nodes, "edges": edges } })
}

fn payload_with_max(nodes: Vec<Value>, edges: Vec<Value>, max_concurrency: Value) -> Value {
    json!({
        "definition": { "nodes": nodes, "edges": edges },
        "maxConcurrency": max_concurrency
    })
}

fn workflow_node(id: &str, workflow_id: &str) -> Value {
    json!({ "id": id, "workflowId": workflow_id })
}

fn edge(source: &str, target: &str) -> Value {
    json!({ "source": source, "target": target })
}

fn create_response(context: &ToolContext, payload: Value) -> Value {
    serde_json::from_str(&pipeline(context, &payload).expect("create pipeline"))
        .expect("parse response")
}

#[test]
#[serial(config)]
fn pipeline_create_smoke_c01_c23_accepts_single_legacy_workflow_node() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let response = create_response(
        &context,
        payload(vec![workflow_node("root", "test-workflow-a")], vec![]),
    );
    let data = response.get("data").expect("response data");
    let node = data["nodes"]
        .as_array()
        .and_then(|nodes| nodes.first())
        .expect("node");
    let session_id = node["sessionId"].as_str().expect("session id");

    assert_eq!(data["ready"], json!(["root"]));
    assert_eq!(node["nodeId"], "root");
    assert_eq!(node["workflowId"], "test-workflow-a");
    assert!(session_id.starts_with("workflow-") && session_id.ends_with("-root"));

    let (frontmatter, _, _) = read_markdown(&workflow_session_uri(session_id), &context.config)
        .expect("read workflow session");
    let frontmatter = frontmatter.expect("workflow frontmatter");
    assert_eq!(
        frontmatter.get("nodeId").and_then(YamlValue::as_str),
        Some("root")
    );
    assert_eq!(
        frontmatter.get("workflowId").and_then(YamlValue::as_str),
        Some("test-workflow-a")
    );
}

#[test]
#[serial(config)]
fn pipeline_create_smoke_c03_builds_linear_chain() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let response = create_response(
        &context,
        payload(
            vec![
                workflow_node("a", "test-workflow-a"),
                workflow_node("b", "test-workflow-b"),
                workflow_node("c", "test-workflow-a"),
                workflow_node("d", "test-workflow-b"),
            ],
            vec![edge("a", "b"), edge("b", "c"), edge("c", "d")],
        ),
    );
    let data = response.get("data").expect("response data");
    let pipeline_id = data["pipelineId"].as_str().expect("pipeline id");
    let (frontmatter, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read pipeline");
    let definition: PipelineDefinition =
        frontmatter_value(&frontmatter.expect("frontmatter"), "definition").expect("definition");

    assert_eq!(data["ready"], json!(["a"]));
    assert_eq!(definition.nodes.len(), 4);
    assert_eq!(definition.edges.len(), 3);
}

#[test]
#[serial(config)]
fn pipeline_create_smoke_c04_builds_diamond_dag() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let response = create_response(
        &context,
        payload(
            vec![
                workflow_node("a", "test-workflow-a"),
                workflow_node("b", "test-workflow-b"),
                workflow_node("c", "test-workflow-a"),
                workflow_node("d", "test-workflow-b"),
            ],
            vec![
                edge("a", "b"),
                edge("a", "c"),
                edge("b", "d"),
                edge("c", "d"),
            ],
        ),
    );

    assert_eq!(response["data"]["ready"], json!(["a"]));
    assert_eq!(response["data"]["nodes"].as_array().map(Vec::len), Some(4));
}

#[test]
#[serial(config)]
fn pipeline_create_smoke_c05_preserves_mixed_edge_types() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let response = create_response(
        &context,
        payload(
            vec![
                workflow_node("root", "test-workflow-a"),
                workflow_node("child", "test-workflow-b"),
                workflow_node("cleanup", "test-workflow-a"),
            ],
            vec![
                edge("root", "child"),
                json!({ "source": "root", "target": "cleanup", "type": "always" }),
            ],
        ),
    );
    let pipeline_id = response["data"]["pipelineId"]
        .as_str()
        .expect("pipeline id");
    let (frontmatter, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read pipeline");
    let definition: PipelineDefinition =
        frontmatter_value(&frontmatter.expect("frontmatter"), "definition").expect("definition");

    assert!(matches!(definition.edges[0].edge_type, EdgeType::Success));
    assert!(matches!(definition.edges[1].edge_type, EdgeType::Always));
}

#[test]
#[serial(config)]
fn pipeline_create_smoke_c06_persists_max_concurrency_one() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let response = create_response(
        &context,
        payload_with_max(
            vec![workflow_node("solo", "test-workflow-a")],
            vec![],
            json!(1),
        ),
    );
    let pipeline_id = response["data"]["pipelineId"]
        .as_str()
        .expect("pipeline id");
    let (frontmatter, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read pipeline");

    assert_eq!(
        frontmatter
            .expect("frontmatter")
            .get("maxConcurrency")
            .and_then(YamlValue::as_u64),
        Some(1)
    );
}

#[test]
#[serial(config)]
fn pipeline_create_smoke_c08_returns_all_root_nodes_as_ready() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let response = create_response(
        &context,
        payload(
            vec![
                workflow_node("a", "test-workflow-a"),
                workflow_node("b", "test-workflow-b"),
                workflow_node("c", "test-workflow-a"),
            ],
            vec![edge("a", "c"), edge("b", "c")],
        ),
    );

    let mut ready: Vec<String> = response["data"]["ready"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_string())
        .collect();
    ready.sort();

    assert_eq!(ready, vec!["a", "b"]);
}

#[test]
#[serial(config)]
fn pipeline_create_smoke_c27_nid07_accepts_special_character_node_ids() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let special_id = "node:branch/leaf";
    let response = create_response(
        &context,
        payload(vec![workflow_node(special_id, "test-workflow-a")], vec![]),
    );
    let node = response["data"]["nodes"]
        .as_array()
        .and_then(|nodes| nodes.first())
        .expect("node");
    let session_id = node["sessionId"].as_str().expect("session id");

    assert_eq!(response["data"]["ready"], json!([special_id]));
    assert_eq!(node["nodeId"], special_id);
    assert!(
        file_exists(&workflow_session_uri(session_id), &context.config).expect("workflow file")
    );
}

#[test]
#[serial(config)]
fn pipeline_create_smoke_c28_accepts_large_pipeline() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let nodes = (0..21)
        .map(|index| {
            let id = format!("n{index:02}");
            let workflow_id = if index % 2 == 0 {
                "test-workflow-a"
            } else {
                "test-workflow-b"
            };
            workflow_node(&id, workflow_id)
        })
        .collect::<Vec<_>>();
    let edges = (0..20)
        .map(|index| edge(&format!("n{index:02}"), &format!("n{:02}", index + 1)))
        .collect::<Vec<_>>();
    let response = create_response(&context, payload(nodes, edges));

    assert_eq!(response["data"]["ready"], json!(["n00"]));
    assert_eq!(response["data"]["nodes"].as_array().map(Vec::len), Some(21));
}

#[test]
#[serial(config)]
fn pipeline_create_smoke_c29_persists_edge_mapping() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let response = create_response(
        &context,
        payload(
            vec![
                workflow_node("root", "test-workflow-a"),
                workflow_node("child", "test-workflow-b"),
            ],
            vec![json!({
                "source": "root",
                "target": "child",
                "mapping": {
                    "title": "results[0].title",
                    "content": "data.content"
                }
            })],
        ),
    );
    let pipeline_id = response["data"]["pipelineId"]
        .as_str()
        .expect("pipeline id");
    let (frontmatter, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read pipeline");
    let definition: PipelineDefinition =
        frontmatter_value(&frontmatter.expect("frontmatter"), "definition").expect("definition");
    let mapping = definition.edges[0].mapping.as_ref().expect("edge mapping");

    assert_eq!(
        mapping.get("title").map(String::as_str),
        Some("results[0].title")
    );
    assert_eq!(
        mapping.get("content").map(String::as_str),
        Some("data.content")
    );
}
