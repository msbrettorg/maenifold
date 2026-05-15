use maenifold_core::config::Config;
use maenifold_core::markdown::{Frontmatter, read_markdown, write_markdown};
use maenifold_core::workflow::pipeline::{NodeState, NodeStatus};
use serde_yml::Value as YamlValue;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

struct ConfigGuard;

impl Drop for ConfigGuard {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

#[tokio::test]
async fn seeded_preflight_failure_promotes_when_execute_pipeline_errors() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let root = std::env::current_dir()
        .expect("current dir")
        .join("target")
        .join("pipeline-driver-integration-tests")
        .join(format!("preflight-err-path-{unique}"));
    std::fs::create_dir_all(&root).expect("create test root");
    Config::override_root(&root);
    Config::set_database_path(root.join("memory.db"));
    let _guard = ConfigGuard;
    let config = Config::current();
    config.ensure_directories().expect("ensure directories");

    let session_id = "pipeline-1735689600000-preflight-err-path";
    let workflow_id = "workflow-1735689600000-preflight-err-path";
    write_pipeline_with_preflight_failure(&config, session_id, workflow_id);
    write_workflow_session(&config, session_id, workflow_id);

    let result =
        maenifold_pipeline_driver::run_pipeline_session(config.clone(), session_id, "").await;

    assert!(result.is_err());
    let (frontmatter, _, _) =
        read_markdown(&pipeline_session_uri(session_id), &config).expect("read pipeline");
    let frontmatter = frontmatter.expect("pipeline frontmatter");
    let states: HashMap<String, NodeState> =
        serde_yml::from_value(frontmatter.get("nodeStates").cloned().expect("node states"))
            .expect("deserialize node states");
    assert_eq!(states["prompt-1"].status, NodeStatus::Stuck);
    assert!(
        states["prompt-1"]
            .failure_reason
            .as_deref()
            .unwrap_or_default()
            .contains("fix agent environment configuration")
    );
    assert_eq!(
        frontmatter.get("status").and_then(YamlValue::as_str),
        Some("stuck")
    );
}

fn write_pipeline_with_preflight_failure(config: &Config, session_id: &str, workflow_id: &str) {
    let node_states = HashMap::from([(
        "prompt-1".to_string(),
        NodeState {
            session_id: workflow_id.to_string(),
            status: NodeStatus::Abandoned,
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
            failure_reason: Some(
                "preflight env validation failed for agent 'misconfigured-agent': MISSING_API_KEY from /nonexistent/path (file not found)".to_string(),
            ),
            retry_after: None,
            retry_count: None,
        },
    )]);
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
        YamlValue::String("pipeline".to_string()),
    );
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("active".to_string()),
    );
    frontmatter.insert(
        "nodeStates".to_string(),
        serde_yml::to_value(node_states).expect("states"),
    );
    write_markdown(
        &pipeline_session_uri(session_id),
        Some(&frontmatter),
        "# Pipeline Session\n",
        config,
    )
    .expect("write pipeline");
}

fn write_workflow_session(config: &Config, pipeline_id: &str, workflow_id: &str) {
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String(format!("Workflow Session {workflow_id}")),
    );
    frontmatter.insert(
        "permalink".to_string(),
        YamlValue::String(workflow_id.to_string()),
    );
    frontmatter.insert(
        "type".to_string(),
        YamlValue::String("workflow".to_string()),
    );
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("abandoned".to_string()),
    );
    frontmatter.insert(
        "pipelineId".to_string(),
        YamlValue::String(pipeline_id.to_string()),
    );
    frontmatter.insert(
        "nodeId".to_string(),
        YamlValue::String("prompt-1".to_string()),
    );
    write_markdown(
        &workflow_session_uri(workflow_id),
        Some(&frontmatter),
        "# Workflow Session\n",
        config,
    )
    .expect("write workflow");
}

fn pipeline_session_uri(session_id: &str) -> String {
    format!("memory://pipelines/2025/01/01/{session_id}")
}

fn workflow_session_uri(session_id: &str) -> String {
    format!("memory://thinking/workflow/2025/01/01/{session_id}")
}
