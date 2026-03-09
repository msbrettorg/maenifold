use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

fn isolated_root() -> TempDir {
    TempDir::new().expect("tmp")
}

fn maenifold_cmd(tmp: &TempDir) -> Command {
    // T-0.3g: RTM-047 - avoid deprecated assert_cmd constructor without changing test behavior.
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("maenifold"));
    cmd.env("MAENIFOLD_ROOT", tmp.path())
        .env("MAENIFOLD_DATABASE_PATH", tmp.path().join("memory.db"));
    cmd
}

#[test]
fn write_memory_json_success_includes_uri_and_checksum() {
    let tmp = isolated_root();
    let payload = r#"{"title":"Contract Test","content":"Testing [[memory-boundary]] from Rust."}"#;

    let output = maenifold_cmd(&tmp)
        .args(["--tool", "WriteMemory", "--payload", payload, "--json"])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let value: Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(value["success"], true);
    assert!(
        value["data"]["uri"]
            .as_str()
            .unwrap_or_default()
            .starts_with("memory://")
    );
    assert!(
        !value["data"]["checksum"]
            .as_str()
            .unwrap_or_default()
            .is_empty()
    );
}

#[test]
fn write_memory_markdown_mode_returns_created_memory_file_prefix() {
    let tmp = isolated_root();
    let payload =
        r#"{"title":"Simple Move Test","content":"Markdown output with [[memory-boundary]]."}"#;

    maenifold_cmd(&tmp)
        .args(["--tool", "WriteMemory", "--payload", payload])
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "Created memory FILE: memory://simple-move-test",
        ));
}

// T-0.3h: RTM FR-1 / FR-2 / FR-4 / FR-5 / FR-6 verifies required-property diagnostics stay aligned with the C# CLI envelope.
#[test]
fn write_memory_missing_title_uses_csharp_style_error_message() {
    let tmp = isolated_root();
    let payload = r#"{"content":"no title [[test]]"}"#;

    let output = maenifold_cmd(&tmp)
        .args(["--tool", "WriteMemory", "--payload", payload, "--json"])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();

    let value: Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(value["success"], false, "unexpected output: {value}");
    assert_eq!(value["error"]["code"], "INVALID_ARGUMENT");
    assert_eq!(
        value["error"]["message"].as_str(),
        Some("Error invoking WriteMemory: Required property 'title' is missing from payload.")
    );
}

#[test]
fn write_memory_rejects_traversal_folder_with_path_validation_error() {
    let tmp = isolated_root();
    let payload = r#"{"title":"Traversal Rejection","content":"Traversal protection with [[path-traversal]].","folder":"safe/../escape"}"#;

    let output = maenifold_cmd(&tmp)
        .args(["--tool", "WriteMemory", "--payload", payload, "--json"])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();

    let value: Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(value["success"], false);
    assert_eq!(value["error"]["code"], "PATH_VALIDATION_ERROR");
}

#[test]
fn read_memory_json_returns_content_uri_and_checksum_after_write() {
    let tmp = isolated_root();
    let write_payload =
        r#"{"title":"Read Test","content":"Content with [[read-memory]] concept."}"#;
    let write_output = maenifold_cmd(&tmp)
        .args([
            "--tool",
            "WriteMemory",
            "--payload",
            write_payload,
            "--json",
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let write_value: Value = serde_json::from_slice(&write_output).expect("write json");
    let uri = write_value["data"]["uri"]
        .as_str()
        .expect("uri")
        .to_string();

    let read_payload = format!(r#"{{"identifier":"{uri}","includeChecksum":true}}"#);
    let read_output = maenifold_cmd(&tmp)
        .args(["--tool", "ReadMemory", "--payload", &read_payload, "--json"])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let read_value: Value = serde_json::from_slice(&read_output).expect("read json");
    assert_eq!(read_value["success"], true);
    assert!(
        read_value["data"]["content"]
            .as_str()
            .unwrap_or_default()
            .contains("[[read-memory]]")
    );
    assert_eq!(read_value["data"]["uri"].as_str(), Some(uri.as_str()));
    assert!(
        !read_value["data"]["checksum"]
            .as_str()
            .unwrap_or_default()
            .is_empty()
    );
}

#[test]
fn write_memory_creates_markdown_file_with_frontmatter_timestamps() {
    let tmp = isolated_root();
    let payload = r#"{"title":"Frontmatter Test","content":"Timestamped [[frontmatter]] note.","folder":"memory-tests"}"#;

    maenifold_cmd(&tmp)
        .args(["--tool", "WriteMemory", "--payload", payload])
        .assert()
        .code(0);

    let file = tmp
        .path()
        .join("memory")
        .join("memory-tests")
        .join("frontmatter-test.md");
    let text = fs::read_to_string(file).expect("markdown file");
    assert!(text.starts_with("---\n"));
    assert!(text.contains("created:"));
    assert!(text.contains("modified:"));
}

#[test]
fn write_memory_without_wikilinks_returns_validation_error_json() {
    let tmp = isolated_root();
    let payload = r#"{"title":"No Concepts","content":"This note has no concepts."}"#;

    let output = maenifold_cmd(&tmp)
        .args(["--tool", "WriteMemory", "--payload", payload, "--json"])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();

    let value: Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(value["success"], false);
    assert_eq!(value["error"]["code"], "VALIDATION_ERROR");
}

#[test]
fn read_memory_missing_identifier_returns_not_found_json() {
    let tmp = isolated_root();
    let payload = r#"{"identifier":"memory://does-not-exist","includeChecksum":true}"#;

    let output = maenifold_cmd(&tmp)
        .args(["--tool", "ReadMemory", "--payload", payload, "--json"])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();

    let value: Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(value["success"], false);
    assert_eq!(value["error"]["code"], "NOT_FOUND");
}

#[test]
fn write_memory_rejects_url_encoded_traversal_folder_with_path_validation_error() {
    let tmp = isolated_root();
    let payload = r#"{"title":"Traversal Rejection","content":"Traversal protection with [[path-traversal]].","folder":"%2e%2e/escape"}"#;

    let output = maenifold_cmd(&tmp)
        .args(["--tool", "WriteMemory", "--payload", payload, "--json"])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();

    let value: Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(value["success"], false);
    assert_eq!(value["error"]["code"], "PATH_VALIDATION_ERROR");
}

#[test]
fn write_memory_with_existing_h1_does_not_duplicate_heading() {
    let tmp = isolated_root();
    let payload = r##"{"title":"Duplicate H1 Test","content":"# Existing Heading\n\nContent with [[concept]]"}"##;

    maenifold_cmd(&tmp)
        .args(["--tool", "WriteMemory", "--payload", payload])
        .assert()
        .code(0);

    let file = tmp.path().join("memory").join("duplicate-h1-test.md");
    let text = fs::read_to_string(file).expect("markdown file");
    let h1_count = text
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with("# ") && !trimmed.starts_with("## ")
        })
        .count();

    assert_eq!(h1_count, 1);
    assert!(text.contains("# Existing Heading"));
}

#[test]
fn read_memory_handles_missing_title_in_frontmatter() {
    let tmp = isolated_root();
    let file = tmp
        .path()
        .join("memory")
        .join("memory-tools-tests")
        .join("no-title-test.md");
    fs::create_dir_all(file.parent().expect("parent")).expect("dirs");
    fs::write(
        &file,
        "---\ntype: memory\nstatus: saved\n---\n\n# Test Content\n\nThis is a [[metadata-handling]] without a title in frontmatter.\n",
    )
    .expect("seed file");

    let payload =
        r#"{"identifier":"memory://memory-tools-tests/no-title-test","includeChecksum":true}"#;
    let output = maenifold_cmd(&tmp)
        .args(["--tool", "ReadMemory", "--payload", payload])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(output).expect("utf8");
    assert!(text.contains("# no-title-test"));
    assert!(text.contains("URI: memory://memory-tools-tests/no-title-test"));
    assert!(text.contains("This is a [[metadata-handling]] without a title in frontmatter."));
}

#[test]
fn read_memory_handles_empty_frontmatter() {
    let tmp = isolated_root();
    let file = tmp
        .path()
        .join("memory")
        .join("memory-tools-tests")
        .join("no-frontmatter-test.md");
    fs::create_dir_all(file.parent().expect("parent")).expect("dirs");
    fs::write(
        &file,
        "# Test Without Frontmatter\n\nThis is a [[yaml-frontmatter]] without any frontmatter.\n",
    )
    .expect("seed file");

    let payload = r#"{"identifier":"memory://memory-tools-tests/no-frontmatter-test","includeChecksum":true}"#;
    let output = maenifold_cmd(&tmp)
        .args(["--tool", "ReadMemory", "--payload", payload])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(output).expect("utf8");
    assert!(text.contains("# no-frontmatter-test"));
    assert!(text.contains("This is a [[yaml-frontmatter]] without any frontmatter."));
}

#[test]
fn write_memory_does_not_write_embedding_fields() {
    let tmp = isolated_root();
    let payload = r#"{"title":"Embedding Removal Test","content":"Content with [[vector-embeddings]] concept to ensure write path works."}"#;

    maenifold_cmd(&tmp)
        .args(["--tool", "WriteMemory", "--payload", payload])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("Created memory FILE:"));

    let path = tmp.path().join("memory").join("embedding-removal-test.md");
    let raw = fs::read_to_string(path).expect("raw markdown");

    assert!(!raw.contains("embedding_base64"));
    assert!(!raw.contains("embedding_model"));
    assert!(!raw.contains("embedding_date"));
}

#[test]
fn move_memory_with_path_based_destination_preserves_markdown_extension_contract() {
    let tmp = isolated_root();
    let write_payload = r##"{"title":"Move Extension Test","content":"# Move Extension Test\n\nTesting [[file-operations]] preserves .md extension."}"##;
    maenifold_cmd(&tmp)
        .args(["--tool", "WriteMemory", "--payload", write_payload])
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "Created memory FILE: memory://move-extension-test",
        ));

    let move_payload = r#"{"source":"memory://move-extension-test","destination":"regression-test/Moved Extension Test"}"#;
    maenifold_cmd(&tmp)
        .args(["--tool", "MoveMemory", "--payload", move_payload])
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "Moved memory FILE: memory://move-extension-test",
        ))
        .stdout(predicate::str::contains(
            "memory://regression-test/moved-extension-test",
        ));

    let moved_path = tmp
        .path()
        .join("memory")
        .join("regression-test")
        .join("moved-extension-test.md");
    assert!(moved_path.is_file());

    let read_payload =
        r#"{"identifier":"memory://regression-test/moved-extension-test","includeChecksum":true}"#;
    let read_output = maenifold_cmd(&tmp)
        .args(["--tool", "ReadMemory", "--payload", read_payload])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(read_output).expect("utf8");
    assert!(text.contains("Testing [[file-operations]] preserves .md extension."));
}

#[test]
fn move_memory_with_simple_destination_preserves_existing_behavior() {
    let tmp = isolated_root();
    let write_payload = r##"{"title":"Simple Move Test","content":"# Simple Move Test\n\nTesting [[refactoring]] functionality."}"##;
    maenifold_cmd(&tmp)
        .args(["--tool", "WriteMemory", "--payload", write_payload])
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "Created memory FILE: memory://simple-move-test",
        ));

    let move_payload =
        r#"{"source":"memory://simple-move-test","destination":"Simple Move Renamed"}"#;
    maenifold_cmd(&tmp)
        .args(["--tool", "MoveMemory", "--payload", move_payload])
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "Moved memory FILE: memory://simple-move-test",
        ))
        .stdout(predicate::str::contains("memory://simple-move-renamed"));

    let read_payload = r#"{"identifier":"memory://simple-move-renamed","includeChecksum":true}"#;
    let read_output = maenifold_cmd(&tmp)
        .args(["--tool", "ReadMemory", "--payload", read_payload])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(read_output).expect("utf8");
    assert!(text.contains("Testing [[refactoring]] functionality."));
}
