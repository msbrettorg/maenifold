use super::JockeyUiClient;
use agent_client_protocol::{self as acp, Client};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

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
