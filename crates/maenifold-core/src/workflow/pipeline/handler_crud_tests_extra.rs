use super::*;
use crate::markdown::Frontmatter;
use crate::sessions::pipeline_session_uri;
use crate::util::json::OutputFormat;
use serial_test::serial;

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
    ) -> crate::error::Result<(Option<Frontmatter>, String, String)> {
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
        input_schema: None,
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
