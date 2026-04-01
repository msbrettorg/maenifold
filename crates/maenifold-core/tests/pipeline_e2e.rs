//! Pipeline end-to-end integration tests — TODO e2e-run-completion.
//!
//! Drives the `pipeline` tool through its public API: constructs a 2-node
//! `ToolNode → ToolNode` chain, runs it end-to-end, and asserts that both
//! nodes reach `"completed"` status within a 60-second deadline.
//!
//! A compile-level construction test (`pipeline_types_compile_and_construct`)
//! is also included and runs unconditionally to catch type-level regressions.
//!
//! # Running the live test
//!
//! ```sh
//! cargo test -p maenifold-core --test pipeline_e2e -- --ignored --nocapture
//! ```
//!
//! # Note on `#[ignore]`
//!
//! The e2e run test is marked `#[ignore = "requires live agents"]` per spec.
//! `ToolNode` pipelines invoke local MCP tools and do **not** actually need an
//! external ACP agent binary; the attribute is present to prevent accidental
//! execution in CI against shared state.

use maenifold_core::{
    Config,
    tools::ToolContext,
    util::json::OutputFormat,
    workflow::pipeline::{EdgeType, PipelineDefinition, PipelineEdge, PipelineNode, pipeline},
};
use serde_json::{Value, json};
use std::fs;
use tempfile::TempDir;
use tokio::time::{Duration, sleep, timeout};

// ---------------------------------------------------------------------------
// Test context helpers
// ---------------------------------------------------------------------------

struct ConfigResetGuard;

impl Drop for ConfigResetGuard {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

/// Build a clean, isolated `ToolContext` backed by a temporary directory so
/// the test never touches the live `~/maenifold` installation.
fn isolated_context() -> (TempDir, ToolContext, ConfigResetGuard) {
    let tmp = TempDir::new().expect("create temp dir");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let config = Config::current();
    config.ensure_directories().expect("ensure directories");
    // The pipeline handler scans this directory; must exist even when no
    // Workflow nodes are present in the pipeline.
    fs::create_dir_all(config.assets_path.join("workflows")).expect("create workflows dir");
    let ctx = ToolContext::new(config, OutputFormat::Json);
    (tmp, ctx, ConfigResetGuard)
}

fn do_create_pipeline(ctx: &ToolContext, definition: Value) -> String {
    let response =
        pipeline(ctx, &json!({ "definition": definition })).expect("pipeline create call");
    serde_json::from_str::<Value>(&response)
        .expect("parse create response")["data"]["pipelineId"]
        .as_str()
        .expect("pipelineId present in response")
        .to_string()
}

fn do_view_pipeline(ctx: &ToolContext, pipeline_id: &str) -> Value {
    let response = pipeline(ctx, &json!({ "sessionId": pipeline_id, "view": true }))
        .expect("pipeline view call");
    serde_json::from_str::<Value>(&response)
        .expect("parse view response")["data"]
        .clone()
}

fn find_node<'a>(data: &'a Value, node_id: &str) -> &'a Value {
    data["nodes"]
        .as_array()
        .expect("nodes array in pipeline view")
        .iter()
        .find(|n| n["nodeId"].as_str() == Some(node_id))
        .unwrap_or_else(|| panic!("node {node_id:?} not found in pipeline view"))
}

/// Poll the view endpoint until the pipeline reaches a terminal state, or
/// until 60 seconds elapse (panics on timeout).
async fn wait_for_terminal(ctx: &ToolContext, pipeline_id: &str) -> Value {
    timeout(Duration::from_secs(60), async {
        loop {
            let data = do_view_pipeline(ctx, pipeline_id);
            match data["status"].as_str() {
                Some("completed" | "cancelled" | "abandoned" | "stuck") => return data,
                _ => sleep(Duration::from_millis(200)).await,
            }
        }
    })
    .await
    .expect("pipeline did not reach a terminal state within 60 seconds")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// T-e2e-run-completion: 2-node ToolNode chain must complete end-to-end.
///
/// Uses the built-in `MemoryStatus` and `GetHelp` MCP tools (no ACP agent
/// required). Marked `#[ignore]` per spec to avoid unintentional CI execution.
#[tokio::test]
#[ignore = "requires live agents"]
async fn pipeline_e2e_run_completion() {
    let (_tmp, ctx, _reset) = isolated_context();

    let pipeline_id = do_create_pipeline(
        &ctx,
        json!({
            "nodes": [
                { "id": "node-a", "type": "tool", "toolName": "MemoryStatus" },
                { "id": "node-b", "type": "tool", "toolName": "GetHelp",
                  "toolArgs": { "toolName": "Pipeline" } }
            ],
            "edges": [
                { "source": "node-a", "target": "node-b" }
            ]
        }),
    );

    pipeline(&ctx, &json!({ "sessionId": pipeline_id, "run": true }))
        .expect("pipeline run call");

    let data = wait_for_terminal(&ctx, &pipeline_id).await;

    assert_eq!(
        data["status"].as_str().unwrap_or(""),
        "completed",
        "pipeline should reach completed; full view:\n{data:#}"
    );
    assert_eq!(
        find_node(&data, "node-a")["status"].as_str().unwrap_or(""),
        "completed",
        "node-a should be completed"
    );
    assert_eq!(
        find_node(&data, "node-b")["status"].as_str().unwrap_or(""),
        "completed",
        "node-b should be completed"
    );
    assert!(
        find_node(&data, "node-a")["output"]
            .as_str()
            .is_some_and(|o| !o.trim().is_empty()),
        "node-a (MemoryStatus) should record non-empty output"
    );
    assert!(
        find_node(&data, "node-b")["output"]
            .as_str()
            .is_some_and(|o| o.contains("Pipeline")),
        "node-b (GetHelp) output should reference Pipeline"
    );
}

/// Compile-time proof: the public pipeline types are constructible and
/// round-trip through `serde_json` without any live infrastructure.
///
/// This test is **not** ignored — it runs in every `cargo test` invocation.
#[test]
fn pipeline_types_compile_and_construct() {
    let def = PipelineDefinition {
        nodes: vec![
            PipelineNode::Tool {
                id: "n1".into(),
                tool_name: "MemoryStatus".into(),
                tool_args: None,
                agent: None,
                model: None,
                thought_level: None,
                allowed_tools: None,
                max_turns: None,
                agent_config: None,
            },
            PipelineNode::Tool {
                id: "n2".into(),
                tool_name: "MemoryStatus".into(),
                tool_args: None,
                agent: None,
                model: None,
                thought_level: None,
                allowed_tools: None,
                max_turns: None,
                agent_config: None,
            },
        ],
        edges: vec![PipelineEdge {
            source: "n1".into(),
            target: "n2".into(),
            edge_type: EdgeType::Success,
            mapping: None,
        }],
    };

    assert_eq!(def.nodes.len(), 2, "expected 2 nodes");
    assert_eq!(def.edges.len(), 1, "expected 1 edge");
    assert_eq!(def.nodes[0].id(), "n1");
    assert_eq!(def.nodes[1].id(), "n2");

    // Serialize → deserialize round-trip must be lossless.
    let serialized = serde_json::to_string(&def).expect("serialize PipelineDefinition");
    assert!(serialized.contains("MemoryStatus"), "serialized form must include tool name");
    assert!(serialized.contains("n1"), "serialized form must include id n1");
    assert!(serialized.contains("n2"), "serialized form must include id n2");

    let round_tripped: PipelineDefinition =
        serde_json::from_str(&serialized).expect("deserialize PipelineDefinition");
    assert_eq!(round_tripped.nodes.len(), 2);
    assert_eq!(round_tripped.edges.len(), 1);
    assert_eq!(round_tripped.nodes[0].id(), "n1");
    assert_eq!(round_tripped.nodes[1].id(), "n2");
}
