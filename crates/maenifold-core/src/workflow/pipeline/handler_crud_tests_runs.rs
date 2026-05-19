use super::*;
use crate::sessions::pipeline_session_uri;
use crate::util::json::OutputFormat;
use chrono::DateTime;
use serial_test::serial;
use std::collections::HashMap;

fn state_with_ts(
    node_id: &str,
    status: NodeStatus,
    started_ms: Option<i64>,
    completed_ms: Option<i64>,
) -> (String, NodeState) {
    (
        node_id.to_string(),
        NodeState {
            session_id: format!("workflow-{TS}-{node_id}"),
            status,
            attempts: 1,
            started_at: started_ms.map(|ms| DateTime::from_timestamp_millis(ms).expect("valid ts")),
            completed_at: completed_ms
                .map(|ms| DateTime::from_timestamp_millis(ms).expect("valid ts")),
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
        },
    )
}

fn seed_db(context: &crate::tools::ToolContext, session_id: &str, status: &str) {
    crate::db::GraphDatabase::initialize_database(&context.config).expect("init db");
    let conn = crate::db::open_connection(&context.config.database_path).expect("open db");
    conn.execute(
        "INSERT INTO file_content (file_path, status) VALUES (?1, ?2)",
        rusqlite::params![pipeline_session_uri(session_id), status],
    )
    .expect("insert row");
}

fn find_entry<'a>(payload: &'a Value, pipeline_id: &str) -> Option<&'a Value> {
    payload["data"]["pipelines"]
        .as_array()?
        .iter()
        .find(|e| e["pipelineId"] == pipeline_id)
}

#[test]
#[serial]
fn list_pipelines_runs_three_completed_nodes() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-runs-three-completed";
    let definition = PipelineDefinition {
        nodes: vec![tool_node("a"), tool_node("b"), tool_node("c")],
        edges: vec![],
        inputs: None,
        input_schema: None,
        template_id: None,
    };
    let start = 1_735_689_600_000_i64;
    let end = start + 60_000_i64;
    let node_states: HashMap<String, NodeState> = [
        state_with_ts("a", NodeStatus::Completed, Some(start), Some(end)),
        state_with_ts(
            "b",
            NodeStatus::Completed,
            Some(start + 1_000),
            Some(end + 1_000),
        ),
        state_with_ts(
            "c",
            NodeStatus::Completed,
            Some(start + 2_000),
            Some(end + 2_000),
        ),
    ]
    .into_iter()
    .collect();
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states, 1);

    let response = list_pipelines(&context).expect("list pipelines");
    let payload: Value = serde_json::from_str(&response).expect("json");
    let entry = find_entry(&payload, session_id).expect("pipeline entry");
    let runs = entry["runs"].as_array().expect("runs must be an array");

    assert_eq!(runs.len(), 3, "expected 3 run entries");
    for run in runs {
        assert!(run["startedAt"].is_number(), "startedAt should be a number");
        assert!(
            run["completedAt"].is_number(),
            "completedAt should be a number"
        );
        assert_eq!(run["status"].as_str(), Some("completed"));
        assert_eq!(run["outcome"].as_str(), Some("succeeded"));
    }
}

#[test]
#[serial]
fn list_pipelines_runs_mixed_status_nodes() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-runs-mixed-status";
    let definition = PipelineDefinition {
        nodes: vec![
            tool_node("done"),
            tool_node("running"),
            tool_node("waiting"),
        ],
        edges: vec![],
        inputs: None,
        input_schema: None,
        template_id: None,
    };
    let start = 1_735_689_600_000_i64;
    let node_states: HashMap<String, NodeState> = [
        state_with_ts(
            "done",
            NodeStatus::Completed,
            Some(start),
            Some(start + 60_000),
        ),
        state_with_ts("running", NodeStatus::Active, Some(start + 1_000), None),
        state_with_ts("waiting", NodeStatus::Pending, None, None),
    ]
    .into_iter()
    .collect();
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states, 1);

    let response = list_pipelines(&context).expect("list pipelines");
    let payload: Value = serde_json::from_str(&response).expect("json");
    let entry = find_entry(&payload, session_id).expect("pipeline entry");
    let runs = entry["runs"].as_array().expect("runs must be an array");
    assert_eq!(runs.len(), 3, "expected 3 run entries");

    let find = |node_id: &str| runs.iter().find(|r| r["nodeId"] == node_id).expect(node_id);

    let done_run = find("done");
    assert_eq!(done_run["status"].as_str(), Some("completed"));
    assert_eq!(done_run["outcome"].as_str(), Some("succeeded"));

    let running_run = find("running");
    assert_eq!(running_run["status"].as_str(), Some("active"));
    assert!(
        running_run["outcome"].is_null(),
        "active node has no outcome"
    );

    let waiting_run = find("waiting");
    assert_eq!(waiting_run["status"].as_str(), Some("pending"));
    assert!(
        waiting_run["outcome"].is_null(),
        "pending node has no outcome"
    );
}

#[test]
#[serial]
fn list_pipelines_runs_pending_node_appears_in_runs() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-runs-pending-node";
    let definition = PipelineDefinition {
        nodes: vec![tool_node("x")],
        edges: vec![],
        inputs: None,
        input_schema: None,
        template_id: None,
    };
    let node_states = states(&[("x", NodeStatus::Pending, 0)]);
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states, 1);

    let response = list_pipelines(&context).expect("list pipelines");
    let payload: Value = serde_json::from_str(&response).expect("json");
    let entry = find_entry(&payload, session_id).expect("pipeline entry");
    let runs = entry["runs"].as_array().expect("runs must be an array");

    assert_eq!(runs.len(), 1, "pending node still appears in runs");
    assert_eq!(runs[0]["status"].as_str(), Some("pending"));
    assert!(
        runs[0]["startedAt"].is_null(),
        "pending node has no startedAt"
    );
    assert!(runs[0]["outcome"].is_null(), "pending node has no outcome");
}

#[test]
#[serial]
fn list_pipelines_degraded_fallback_has_empty_runs() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-runs-degraded";
    std::fs::create_dir_all(
        context
            .config
            .memory_path
            .join("pipelines")
            .join(format!("{session_id}.md")),
    )
    .expect("create dir-as-file to force unreadable");
    seed_db(&context, session_id, "completed");

    let response = list_pipelines(&context).expect("list pipelines");
    let payload: Value = serde_json::from_str(&response).expect("json");
    // Degraded entries use the full URI as pipelineId, not the bare session_id.
    let pipeline_uri = pipeline_session_uri(session_id);
    let entry = payload["data"]["pipelines"]
        .as_array()
        .expect("pipelines array")
        .iter()
        .find(|e| e["pipelineId"] == pipeline_uri)
        .expect("degraded pipeline entry");
    let runs = entry["runs"]
        .as_array()
        .expect("degraded entry must have runs array");
    assert!(runs.is_empty(), "degraded entry runs must be empty");
}
