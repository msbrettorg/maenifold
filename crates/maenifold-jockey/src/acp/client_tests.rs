use super::super::worker::AcpEvent;
use super::JockeyUiClient;
use agent_client_protocol::schema::{self as acp};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

fn test_root(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("acp-fs-tests")
        .join(format!("{name}-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn client(root: &Path) -> JockeyUiClient {
    JockeyUiClient {
        delta_slot: Arc::new(Mutex::new(None)),
        auto_approve: false,
        workspace_root: root.to_path_buf(),
    }
}

fn client_with_delta(root: &Path) -> (JockeyUiClient, mpsc::Receiver<AcpEvent>) {
    let (tx, rx) = mpsc::channel(32);
    (
        JockeyUiClient {
            delta_slot: Arc::new(Mutex::new(Some(tx))),
            auto_approve: false,
            workspace_root: root.to_path_buf(),
        },
        rx,
    )
}

fn text_content(text: &str) -> acp::ToolCallContent {
    acp::ToolCallContent::from(acp::ContentBlock::Text(acp::TextContent::new(text)))
}

fn terminal_blocks(content: &[Value]) -> Vec<&Value> {
    content
        .iter()
        .filter(|item| item.get("type").and_then(Value::as_str) == Some("terminal"))
        .collect()
}

#[tokio::test]
async fn rejects_non_absolute_read_path() {
    let root = test_root("non-absolute");
    let result = client(&root)
        .read_text_file(acp::ReadTextFileRequest::new("s", "relative.txt"))
        .await;
    assert!(result.unwrap_err().to_string().contains("absolute"));
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn rejects_paths_outside_workspace() {
    let root = test_root("containment");
    let outside = root
        .parent()
        .unwrap()
        .join(format!("outside-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&outside).unwrap();
    let outside_file = outside.join("secret.txt");
    std::fs::write(&outside_file, "secret").unwrap();

    let result = client(&root)
        .read_text_file(acp::ReadTextFileRequest::new("s", outside_file))
        .await;
    assert!(result.unwrap_err().to_string().contains("escapes"));
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(outside);
}

#[cfg(unix)]
#[tokio::test]
async fn rejects_symlink_traversal_outside_workspace() {
    let root = test_root("symlink");
    let outside = root
        .parent()
        .unwrap()
        .join(format!("symlink-outside-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("secret.txt"), "secret").unwrap();
    std::os::unix::fs::symlink(&outside, root.join("link")).unwrap();

    let result = client(&root)
        .read_text_file(acp::ReadTextFileRequest::new(
            "s",
            root.join("link").join("secret.txt"),
        ))
        .await;
    assert!(result.unwrap_err().to_string().contains("escapes"));
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(outside);
}

#[cfg(unix)]
#[tokio::test]
async fn rejects_write_through_dangling_symlink() {
    let root = test_root("dangling-symlink");
    let outside = root
        .parent()
        .unwrap()
        .join(format!("dangling-outside-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&outside).unwrap();
    let outside_file = outside.join("future.txt");
    let link = root.join("link-file");
    std::os::unix::fs::symlink(&outside_file, &link).unwrap();

    let result = client(&root)
        .write_text_file(acp::WriteTextFileRequest::new("s", &link, "content"))
        .await;
    assert!(result.unwrap_err().to_string().contains("resolve"));
    assert!(!outside_file.exists());
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(outside);
}

#[tokio::test]
async fn write_parent_creation_requires_explicit_meta_flag() {
    let root = test_root("create-parent");
    let path = root.join("missing").join("file.txt");
    let result = client(&root)
        .write_text_file(acp::WriteTextFileRequest::new("s", &path, "content"))
        .await;
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("createIntermediates")
    );

    let mut meta = acp::Meta::new();
    meta.insert("createIntermediates".into(), Value::Bool(true));
    client(&root)
        .write_text_file(acp::WriteTextFileRequest::new("s", &path, "content").meta(meta))
        .await
        .unwrap();
    assert_eq!(std::fs::read_to_string(path).unwrap(), "content");
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn rejects_parent_components_before_creating_intermediates() {
    let root = test_root("parent-component");
    let outside_name = format!("outside-parent-{}", uuid::Uuid::new_v4());
    let outside = root.parent().unwrap().join(&outside_name);
    let path = root
        .join("missing")
        .join("..")
        .join("..")
        .join(&outside_name)
        .join("file.txt");
    let mut meta = acp::Meta::new();
    meta.insert("createIntermediates".into(), Value::Bool(true));

    let result = client(&root)
        .write_text_file(acp::WriteTextFileRequest::new("s", &path, "content").meta(meta))
        .await;

    assert!(result.unwrap_err().to_string().contains(".."));
    assert!(!outside.exists());
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn execute_text_content_translated_to_terminal() {
    let root = test_root("execute-terminal");
    let (client, mut rx) = client_with_delta(&root);
    let update = acp::SessionUpdate::ToolCall(
        acp::ToolCall::new("tc-exec-1", "run ls")
            .kind(acp::ToolKind::Execute)
            .status(acp::ToolCallStatus::Completed)
            .content(vec![text_content("ls -la output...")]),
    );

    client
        .session_notification(acp::SessionNotification::new("sess-1", update))
        .await
        .unwrap();
    let event = rx.recv().await.unwrap();

    let AcpEvent::ToolCall {
        content: Some(content),
        ..
    } = event
    else {
        panic!("expected ToolCall event with content");
    };
    let terminals = terminal_blocks(&content);
    assert!(!terminals.is_empty(), "expected synthesized terminal block");
    let terminal_id = terminals[0]
        .get("terminalId")
        .and_then(Value::as_str)
        .expect("terminal block should include terminalId");
    assert!(terminal_id.contains("tc-exec-1"));
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn execute_update_text_content_translated_to_terminal() {
    let root = test_root("execute-update-terminal");
    let (client, mut rx) = client_with_delta(&root);
    let update = acp::SessionUpdate::ToolCallUpdate(acp::ToolCallUpdate::new(
        "tc-exec-update-1",
        acp::ToolCallUpdateFields::new()
            .kind(acp::ToolKind::Execute)
            .status(acp::ToolCallStatus::Completed)
            .content(vec![text_content("cargo build output...")]),
    ));

    client
        .session_notification(acp::SessionNotification::new("sess-1", update))
        .await
        .unwrap();
    let event = rx.recv().await.unwrap();

    let AcpEvent::ToolCallUpdate {
        content: Some(content),
        ..
    } = event
    else {
        panic!("expected ToolCallUpdate event with content");
    };
    let terminals = terminal_blocks(&content);
    assert!(!terminals.is_empty(), "expected synthesized terminal block");
    let terminal_id = terminals[0]
        .get("terminalId")
        .and_then(Value::as_str)
        .expect("terminal block should include terminalId");
    assert!(terminal_id.contains("tc-exec-update-1"));
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn non_execute_text_content_not_translated() {
    let root = test_root("read-content");
    let (client, mut rx) = client_with_delta(&root);
    let update = acp::SessionUpdate::ToolCall(
        acp::ToolCall::new("tc-read-1", "read file")
            .kind(acp::ToolKind::Read)
            .status(acp::ToolCallStatus::Completed)
            .content(vec![text_content("file contents...")]),
    );

    client
        .session_notification(acp::SessionNotification::new("sess-1", update))
        .await
        .unwrap();
    let event = rx.recv().await.unwrap();

    let AcpEvent::ToolCall {
        content: Some(content),
        ..
    } = event
    else {
        panic!("expected ToolCall event with content");
    };
    assert_eq!(content.len(), 1);
    assert_eq!(
        content[0].get("type").and_then(Value::as_str),
        Some("content")
    );
    assert!(terminal_blocks(&content).is_empty());
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn execute_with_existing_terminal_content_not_double_translated() {
    let root = test_root("existing-terminal");
    let (client, mut rx) = client_with_delta(&root);
    let update = acp::SessionUpdate::ToolCall(
        acp::ToolCall::new("tc-exec-existing-1", "run command")
            .kind(acp::ToolKind::Execute)
            .status(acp::ToolCallStatus::Completed)
            .content(vec![acp::ToolCallContent::Terminal(acp::Terminal::new(
                "existing-term",
            ))]),
    );

    client
        .session_notification(acp::SessionNotification::new("sess-1", update))
        .await
        .unwrap();
    let event = rx.recv().await.unwrap();

    let AcpEvent::ToolCall {
        content: Some(content),
        ..
    } = event
    else {
        panic!("expected ToolCall event with content");
    };
    let terminals = terminal_blocks(&content);
    assert_eq!(terminals.len(), 1);
    assert_eq!(
        terminals[0].get("terminalId").and_then(Value::as_str),
        Some("existing-term")
    );
    let _ = std::fs::remove_dir_all(root);
}
