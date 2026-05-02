use super::*;
use crate::markdown::{delete_file, file_exists, read_markdown, write_markdown};
use crate::sessions::{pipeline_session_uri, workflow_session_uri};
use crate::util::json::OutputFormat;
use serial_test::serial;
fn set_stored_pipeline_status(context: &ToolContext, session_id: &str, status: &str) {
    let uri = pipeline_session_uri(session_id);
    let (frontmatter, content, _) =
        read_markdown(&uri, &context.config).expect("read pipeline markdown");
    let mut frontmatter = frontmatter.expect("pipeline frontmatter");
    frontmatter.insert("status".to_string(), status.into());
    write_markdown(&uri, Some(&frontmatter), &content, &context.config)
        .expect("write pipeline status");
}

#[test]
#[serial]
fn delete_pipeline_falls_back_to_raw_node_states_when_derived_state_is_corrupt() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-delete-corrupt-derived-state";
    let definition = PipelineDefinition {
        nodes: vec![workflow_node("node")],
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    let node_states = states(&[("node", NodeStatus::Completed, 1)]);
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states.clone(), 1);

    delete_file(
        &workflow_session_uri(&node_states["node"].session_id),
        &context.config,
    )
    .expect("delete workflow session to corrupt derived state");

    let response =
        delete_pipeline(&context, session_id).expect("delete should tolerate corruption");
    let payload: Value = serde_json::from_str(&response).expect("json response");

    assert_eq!(payload["success"], Value::Bool(true));
    assert_eq!(
        payload["data"]["deletedSessionId"],
        Value::String(session_id.to_string())
    );
    assert_eq!(
        payload["data"]["cleanedUpFiles"],
        Value::Number(serde_json::Number::from(1)),
    );
    assert_eq!(
        payload["data"]["skippedFiles"],
        Value::Number(serde_json::Number::from(1)),
    );
    assert!(
        !file_exists(&pipeline_session_uri(session_id), &context.config)
            .expect("check pipeline deletion"),
        "pipeline session should be deleted"
    );
}

#[test]
#[serial]
fn delete_pipeline_completed_status_rejected_without_force() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-delete-completed-no-force";
    let definition = PipelineDefinition {
        nodes: vec![workflow_node("node")],
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    let node_states = states(&[("node", NodeStatus::Completed, 1)]);
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states, 1);
    set_stored_pipeline_status(&context, session_id, "completed");

    let error = delete_pipeline(&context, session_id).expect_err("completed delete needs force");

    assert!(
        error.to_string().contains("is completed"),
        "expected completed-pipeline refusal, got: {error}"
    );
    assert!(
        file_exists(&pipeline_session_uri(session_id), &context.config)
            .expect("check pipeline preservation"),
        "pipeline session should remain after refused delete"
    );
}

#[test]
#[serial]
fn delete_pipeline_completed_status_succeeds_with_force_true() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-delete-completed-force";
    let definition = PipelineDefinition {
        nodes: vec![workflow_node("node")],
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    let node_states = states(&[("node", NodeStatus::Completed, 1)]);
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states, 1);
    set_stored_pipeline_status(&context, session_id, "completed");

    let response =
        delete_pipeline_with_force(&context, session_id, true).expect("force delete completed");
    let payload: Value = serde_json::from_str(&response).expect("json response");

    assert_eq!(payload["success"], Value::Bool(true));
    assert!(
        !file_exists(&pipeline_session_uri(session_id), &context.config)
            .expect("check pipeline deletion"),
        "pipeline session should be deleted with force"
    );
}

#[test]
#[serial]
fn delete_pipeline_running_or_failed_status_does_not_require_force() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-delete-failed-no-force";
    let definition = PipelineDefinition {
        nodes: vec![workflow_node("node")],
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    let node_states = states(&[("node", NodeStatus::Completed, 1)]);
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states, 1);
    set_stored_pipeline_status(&context, session_id, "failed");

    let response = delete_pipeline(&context, session_id).expect("failed delete without force");
    let payload: Value = serde_json::from_str(&response).expect("json response");

    assert_eq!(payload["success"], Value::Bool(true));
    assert!(
        !file_exists(&pipeline_session_uri(session_id), &context.config)
            .expect("check pipeline deletion"),
        "failed pipeline session should be deleted without force"
    );
}

#[test]
#[serial]
fn delete_pipeline_cleans_up_historical_orphaned_child_workflow_sessions() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-delete-orphaned-child-workflows";
    let definition = PipelineDefinition {
        nodes: vec![workflow_node("node")],
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    let node_states = states(&[("node", NodeStatus::Completed, 1)]);
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states.clone(), 1);

    let orphan_session_id = "workflow-1735689600000-node-orphan";
    let (frontmatter, content, _) = read_markdown(
        &workflow_session_uri(&node_states["node"].session_id),
        &context.config,
    )
    .expect("read workflow session");
    let mut orphan_frontmatter = frontmatter.expect("workflow frontmatter");
    orphan_frontmatter.insert(
        "title".to_string(),
        format!("Workflow Session {orphan_session_id}").into(),
    );
    orphan_frontmatter.insert("permalink".to_string(), orphan_session_id.into());
    orphan_frontmatter.insert("nodeId".to_string(), "node-orphan".into());
    write_markdown(
        &workflow_session_uri(orphan_session_id),
        Some(&orphan_frontmatter),
        &content,
        &context.config,
    )
    .expect("write orphaned workflow session");

    let response = delete_pipeline(&context, session_id).expect("delete pipeline");
    let payload: Value = serde_json::from_str(&response).expect("json response");

    assert_eq!(payload["success"], Value::Bool(true));
    assert_eq!(
        payload["data"]["cleanedUpFiles"],
        Value::Number(serde_json::Number::from(3)),
    );
    assert!(
        !file_exists(&workflow_session_uri(orphan_session_id), &context.config)
            .expect("check orphan workflow deletion"),
        "historical orphaned workflow session should be deleted"
    );
}

#[test]
#[serial]
fn delete_pipeline_does_not_delete_sibling_pipeline_child_workflow_sessions() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let target_session_id = "pipeline-1735689600000-delete-target-pipeline";
    let sibling_session_id = "pipeline-1735689600000-delete-sibling-pipeline";
    let definition = PipelineDefinition {
        nodes: vec![workflow_node("target-node")],
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    let sibling_definition = PipelineDefinition {
        nodes: vec![workflow_node("sibling-node")],
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    let target_node_states = states(&[("target-node", NodeStatus::Completed, 1)]);
    let sibling_node_states = states(&[("sibling-node", NodeStatus::Completed, 1)]);
    seed_pipeline_session_with_workflows(
        &context,
        target_session_id,
        definition,
        target_node_states.clone(),
        1,
    );
    seed_pipeline_session_with_workflows(
        &context,
        sibling_session_id,
        sibling_definition,
        sibling_node_states.clone(),
        1,
    );

    let sibling_orphan_session_id = "workflow-1735689600000-sibling-orphan";
    let (frontmatter, content, _) = read_markdown(
        &workflow_session_uri(&sibling_node_states["sibling-node"].session_id),
        &context.config,
    )
    .expect("read sibling workflow session");
    let mut sibling_orphan_frontmatter = frontmatter.expect("sibling workflow frontmatter");
    sibling_orphan_frontmatter.insert(
        "title".to_string(),
        format!("Workflow Session {sibling_orphan_session_id}").into(),
    );
    sibling_orphan_frontmatter.insert("permalink".to_string(), sibling_orphan_session_id.into());
    sibling_orphan_frontmatter.insert("nodeId".to_string(), "sibling-orphan".into());
    write_markdown(
        &workflow_session_uri(sibling_orphan_session_id),
        Some(&sibling_orphan_frontmatter),
        &content,
        &context.config,
    )
    .expect("write sibling orphaned workflow session");

    let response = delete_pipeline(&context, target_session_id).expect("delete target pipeline");
    let payload: Value = serde_json::from_str(&response).expect("json response");

    assert_eq!(payload["success"], Value::Bool(true));
    assert_eq!(
        payload["data"]["cleanedUpFiles"],
        Value::Number(serde_json::Number::from(2)),
    );
    assert!(
        file_exists(
            &workflow_session_uri(&sibling_node_states["sibling-node"].session_id),
            &context.config
        )
        .expect("check sibling workflow preservation"),
        "sibling pipeline workflow session should be preserved"
    );
    assert!(
        file_exists(
            &workflow_session_uri(sibling_orphan_session_id),
            &context.config
        )
        .expect("check sibling orphan preservation"),
        "sibling pipeline orphaned workflow session should be preserved"
    );
}
fn seed_db_row(context: &crate::tools::ToolContext, session_id: &str, status: &str) {
    crate::db::GraphDatabase::initialize_database(&context.config).expect("init db");
    let conn = crate::db::open_connection(&context.config.database_path).expect("open db");
    conn.execute(
        "INSERT INTO file_content (file_path, status) VALUES (?1, ?2)",
        rusqlite::params![pipeline_session_uri(session_id), status],
    )
    .expect("insert row");
}
fn pipeline_entry<'a>(payload: &'a Value, pipeline_id: &str) -> Option<&'a Value> {
    payload["data"]["pipelines"]
        .as_array()?
        .iter()
        .find(|entry| entry["pipelineId"] == pipeline_id)
}
#[test]
#[serial]
fn list_pipelines_emits_degraded_entry_when_markdown_unreadable() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-list-unreadable";
    std::fs::create_dir_all(
        context
            .config
            .memory_path
            .join("pipelines")
            .join(format!("{session_id}.md")),
    )
    .expect("unreadable dir");
    seed_db_row(&context, session_id, "completed");
    let payload: Value =
        serde_json::from_str(&list_pipelines(&context).expect("list")).expect("json");
    let entry =
        pipeline_entry(&payload, &pipeline_session_uri(session_id)).expect("degraded entry");
    assert_eq!(entry["status"], "completed");
    assert_eq!(entry["nodeCount"], Value::Null);
    assert_eq!(entry["completedCount"], Value::Null);
    assert_eq!(entry["degraded"]["source"], "db");
    assert_eq!(entry["degraded"]["reason"], "markdown_unreadable");
}
#[test]
#[serial]
fn list_pipelines_emits_dataless_reason_when_file_dataless() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    fn dataless(
        _: &str,
        _: &crate::config::Config,
    ) -> crate::error::Result<(Option<crate::markdown::Frontmatter>, String, String)> {
        Err(crate::error::MaenifoldError::FileDataless {
            path: std::path::PathBuf::from("dataless.md"),
            size: 64,
        })
    }
    let session_id = "pipeline-1735689600000-list-dataless";
    std::fs::create_dir_all(context.config.memory_path.join("pipelines")).expect("pipeline dir");
    std::fs::write(
        context
            .config
            .memory_path
            .join("pipelines")
            .join(format!("{session_id}.md")),
        "x",
    )
    .expect("pipeline file");
    seed_db_row(&context, session_id, "completed");
    let payload: Value =
        serde_json::from_str(&list_pipelines_impl(&context, dataless).expect("list"))
            .expect("json");
    let entry =
        pipeline_entry(&payload, &pipeline_session_uri(session_id)).expect("degraded entry");
    assert_eq!(entry["degraded"]["source"], "db");
    assert_eq!(entry["degraded"]["reason"], "dataless");
}
#[test]
#[serial]
fn list_pipelines_skips_when_neither_markdown_nor_db() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-list-missing-db";
    std::fs::create_dir_all(
        context
            .config
            .memory_path
            .join("pipelines")
            .join(format!("{session_id}.md")),
    )
    .expect("unreadable dir");
    let payload: Value =
        serde_json::from_str(&list_pipelines(&context).expect("list")).expect("json");
    assert!(pipeline_entry(&payload, &pipeline_session_uri(session_id)).is_none());
}
#[test]
#[serial]
fn list_pipelines_normal_path_unchanged() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-list-normal";
    let definition = PipelineDefinition {
        nodes: vec![workflow_node("node")],
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    seed_pipeline_session_with_workflows(
        &context,
        session_id,
        definition,
        states(&[("node", NodeStatus::Completed, 1)]),
        1,
    );
    let payload: Value =
        serde_json::from_str(&list_pipelines(&context).expect("list")).expect("json");
    let entry = pipeline_entry(&payload, session_id).expect("normal entry");
    assert_eq!(entry["status"], "active");
    assert_eq!(entry["nodeCount"], 1);
    assert_eq!(entry["completedCount"], 1);
    assert!(entry.get("degraded").is_none());
}
