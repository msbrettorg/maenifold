use super::gateway_catalog_domain_tests::native_catalog;
use super::gateway_catalog_test_fixtures::*;
use crate::mcp::gateway_gate;
use crate::util::paths::slugify_title;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

#[tokio::test]
async fn gate_create_command_is_listed_in_maenifold_gate_domain_catalog() {
    let catalog = native_catalog().await;
    let commands = catalog.commands("maenifold.gate").expect("gate domain");
    assert!(
        commands.iter().any(|command| command.name == "gate_Create"),
        "gate_Create missing from maenifold.gate catalog: {:?}",
        commands.iter().map(|command| &command.name).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn gate_create_happy_path_writes_frontmatter_and_auto_syncs_status() {
    let root = gate_root();
    let title = "Create the gateway task";
    let result = invoke_create(
        root.path(),
        "T-99.1",
        title,
        "tdd",
        json!({ "body": "## Problem\n\nNeed a gateway create command.\n" }),
    )
    .await;

    assert!(!result.is_error.unwrap_or(false), "{}", text(&result));
    let rendered = parsed_json(&result);
    let expected_path = root
        .path()
        .join(format!("todo/T-99.1-{}.md", slugify_title(title)));

    assert_eq!(rendered["taskId"], "T-99.1");
    assert_eq!(rendered["pipeline"], "tdd");
    assert_eq!(rendered["sprint"], 99);
    assert_eq!(
        rendered["path"].as_str().map(PathBuf::from),
        Some(expected_path.clone())
    );

    let markdown = fs::read_to_string(&expected_path).expect("created task file");
    let frontmatter = frontmatter(&markdown);
    assert_eq!(frontmatter["id"], "T-99.1");
    assert_eq!(frontmatter["title"], title);
    assert_eq!(frontmatter["pipeline"], "tdd");
    assert_eq!(frontmatter["priority"], "P2");
    assert_eq!(frontmatter["sprint"], 99);

    let status = gateway_gate::invoke_gate(
        "gate_Status".to_string(),
        json!({ "root": root.path().display().to_string(), "includeDone": true }),
    )
    .await
    .expect("status result");
    assert!(!status.is_error.unwrap_or(false), "{}", text(&status));
    let rows = parsed_json(&status).as_array().cloned().expect("status rows");
    assert!(
        rows.iter().any(|row| row["taskId"] == "T-99.1"),
        "gate_Status should surface newly-created task: {rows:?}"
    );
}

#[tokio::test]
async fn gate_create_rejects_duplicate_task_ids_in_todo_and_done() {
    for location in ["todo", "done"] {
        let root = gate_root();
        write_existing_task(root.path(), location, "T-99.1", "existing duplicate", "tdd");
        let result = invoke_create(root.path(), "T-99.1", "duplicate", "tdd", json!({})).await;

        assert!(result.is_error.unwrap_or(false), "{location} duplicate should fail");
        let message = text(&result);
        assert!(message.contains("T-99.1"), "{message}");
        assert!(
            message.contains("already exists") || message.contains("duplicate"),
            "{message}"
        );
        assert!(message.contains(location), "{message}");
    }
}

#[tokio::test]
async fn gate_create_rejects_invalid_task_id_inputs() {
    for invalid in ["foo", "T-1", "T-1.2.3", "", " T-1.2", "t-1.2"] {
        let root = gate_root();
        let result = invoke_create(root.path(), invalid, "invalid id", "tdd", json!({})).await;

        assert!(result.is_error.unwrap_or(false), "{invalid:?} should fail");
        let message = text(&result);
        assert!(message.contains("taskId") || message.contains("task id"), "{message}");
        assert!(message.contains("T-") || message.contains("regex"), "{message}");
    }
}

#[tokio::test]
async fn gate_create_rejects_unknown_pipeline_and_invalid_priority_values() {
    let root = gate_root();
    let unknown = invoke_create(
        root.path(),
        "T-99.1",
        "unknown pipeline",
        "frobnicate",
        json!({}),
    )
    .await;
    assert!(unknown.is_error.unwrap_or(false));
    let pipeline_message = text(&unknown);
    assert!(pipeline_message.contains("frobnicate"), "{pipeline_message}");
    for expected in ["sdd", "tdd", "gsd", "lightweight"] {
        assert!(pipeline_message.contains(expected), "{pipeline_message}");
    }

    let invalid_priority = invoke_create(
        root.path(),
        "T-99.2",
        "bad priority",
        "tdd",
        json!({ "priority": "PX" }),
    )
    .await;
    assert!(invalid_priority.is_error.unwrap_or(false));
    let priority_message = text(&invalid_priority);
    assert!(priority_message.contains("priority"), "{priority_message}");
    assert!(
        priority_message.contains("P0") || priority_message.contains("P4"),
        "{priority_message}"
    );
}

#[tokio::test]
async fn gate_create_rejects_empty_and_whitespace_titles() {
    for title in ["", "   "] {
        let root = gate_root();
        let result = invoke_create(root.path(), "T-99.1", title, "tdd", json!({})).await;
        assert!(result.is_error.unwrap_or(false), "{title:?} should fail");
        let message = text(&result);
        assert!(message.contains("title"), "{message}");
        assert!(
            message.contains("empty") || message.contains("required"),
            "{message}"
        );
    }
}

#[tokio::test]
async fn gate_create_slugifies_unsafe_titles_without_path_escape() {
    for (index, title) in ["foo/bar", "../etc", "foo\u{0000}bar"]
        .into_iter()
        .enumerate()
    {
        let root = gate_root();
        let task_id = format!("T-99.{}", index + 1);
        let result = invoke_create(root.path(), &task_id, title, "tdd", json!({})).await;

        assert!(!result.is_error.unwrap_or(false), "{}", text(&result));
        let rendered = parsed_json(&result);
        let expected = root
            .path()
            .join(format!("todo/{task_id}-{}.md", slugify_title(title)));
        let actual = PathBuf::from(
            rendered["path"]
                .as_str()
                .expect("returned file path should be a string"),
        );

        assert_eq!(actual, expected);
        assert_eq!(actual.parent(), Some(root.path().join("todo").as_path()));
        assert!(actual.starts_with(root.path().join("todo")));
        assert!(actual.exists(), "created path should exist: {}", actual.display());
    }
}

#[tokio::test]
async fn gate_create_accepts_registered_custom_pipeline_and_respects_optional_fields() {
    let root = gate_root();
    write_custom_pipeline(root.path(), "custom-create-pipeline");
    let body = "## Custom Body\n\nShip exactly this body.\n";

    for (index, priority) in ["P0", "P1", "P2", "P3", "P4"].into_iter().enumerate() {
        let task_id = format!("T-99.{}", index + 1);
        let result = invoke_create(
            root.path(),
            &task_id,
            "custom pipeline task",
            "custom-create-pipeline",
            json!({ "body": body, "priority": priority, "sprint": 501, "parent": "T-99.0" }),
        )
        .await;

        assert!(!result.is_error.unwrap_or(false), "{}", text(&result));
        let rendered = parsed_json(&result);
        assert_eq!(rendered["pipeline"], "custom-create-pipeline");
        assert_eq!(rendered["sprint"], 501);

        let markdown = fs::read_to_string(
            rendered["path"]
                .as_str()
                .expect("result path should be a string"),
        )
        .expect("created markdown");
        let yaml = frontmatter(&markdown);
        assert_eq!(yaml["pipeline"], "custom-create-pipeline");
        assert_eq!(yaml["priority"], priority);
        assert_eq!(yaml["sprint"], 501);
        assert_eq!(yaml["parent"], "T-99.0");
        assert!(markdown.contains(body), "provided body should be preserved: {markdown}");
        assert!(!markdown.contains("## Plan"), "{markdown}");
    }
}

#[tokio::test]
async fn gate_create_scaffolds_default_body_and_derives_default_sprint_and_priority() {
    let root = gate_root();
    let result = invoke_create(root.path(), "T-321.7", "scaffold me", "tdd", json!({})).await;

    assert!(!result.is_error.unwrap_or(false), "{}", text(&result));
    let rendered = parsed_json(&result);
    assert_eq!(rendered["sprint"], 321);

    let markdown = fs::read_to_string(
        rendered["path"]
            .as_str()
            .expect("result path should be a string"),
    )
    .expect("created markdown");
    let yaml = frontmatter(&markdown);
    assert_eq!(yaml["priority"], "P2");
    assert_eq!(yaml["sprint"], 321);
    assert!(yaml.get("parent").is_none(), "parent should be omitted");
    for heading in ["## Problem", "## Plan", "## Acceptance", "## References"] {
        assert!(markdown.contains(heading), "{markdown}");
    }
}

#[tokio::test]
async fn gate_create_rejects_missing_root_and_missing_todo_folder() {
    let missing_root = tempfile::tempdir().expect("tempdir");
    let nonexistent = missing_root.path().join("does-not-exist");
    let missing_root_result =
        invoke_create(&nonexistent, "T-99.1", "missing root", "tdd", json!({})).await;
    assert!(missing_root_result.is_error.unwrap_or(false));
    let missing_root_message = text(&missing_root_result);
    assert!(
        missing_root_message.contains("No such file")
            || missing_root_message.contains("does not exist")
            || missing_root_message.contains("canonicalize"),
        "{missing_root_message}"
    );

    let root = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(root.path().join("done")).expect("done dir");
    let missing_todo_result =
        invoke_create(root.path(), "T-99.1", "missing todo", "tdd", json!({})).await;
    assert!(missing_todo_result.is_error.unwrap_or(false));
    let missing_todo_message = text(&missing_todo_result);
    assert!(missing_todo_message.contains("todo"), "{missing_todo_message}");
    assert!(
        missing_todo_message.contains("missing") || missing_todo_message.contains("required"),
        "{missing_todo_message}"
    );
}
