use super::*;
use crate::markdown::{delete_file, file_exists, read_markdown, write_markdown};
use crate::sessions::{pipeline_session_uri, workflow_session_uri};
use crate::util::json::OutputFormat;
use serial_test::serial;

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
    seed_pipeline_session_with_workflows(
        &context,
        session_id,
        definition,
        node_states.clone(),
        1,
    );

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
    seed_pipeline_session_with_workflows(
        &context,
        session_id,
        definition,
        node_states.clone(),
        1,
    );

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
    sibling_orphan_frontmatter
        .insert("permalink".to_string(), sibling_orphan_session_id.into());
    sibling_orphan_frontmatter.insert("nodeId".to_string(), "sibling-orphan".into());
    write_markdown(
        &workflow_session_uri(sibling_orphan_session_id),
        Some(&sibling_orphan_frontmatter),
        &content,
        &context.config,
    )
    .expect("write sibling orphaned workflow session");

    let response =
        delete_pipeline(&context, target_session_id).expect("delete target pipeline");
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
