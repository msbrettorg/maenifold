// T-12.7: RTM NFR-1 - keep memory boundary CLI coverage split by write/read contract theme.
#[path = "support/memory_boundary_cli.rs"]
mod memory_boundary_cli_support;

use memory_boundary_cli_support::{isolated_root, maenifold_cmd};
use predicates::prelude::*;
use serde_json::Value;
use std::fs;

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
