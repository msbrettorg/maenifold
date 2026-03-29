// T-12.7: RTM NFR-1 - keep memory boundary CLI coverage split by metadata and move semantics.
#[path = "support/memory_boundary_cli.rs"]
mod memory_boundary_cli_support;

use memory_boundary_cli_support::{isolated_root, maenifold_cmd};
use predicates::prelude::*;
use std::fs;

#[test]
fn write_memory_with_existing_h1_does_not_duplicate_heading() {
    let tmp = isolated_root();
    let payload = r##"{"title":"Duplicate H1 Test","content":"# Existing Heading\n\nContent with [[concept]]"}"##;

    maenifold_cmd(&tmp)
        .args(["tool", "WriteMemory", payload])
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
        .args(["tool", "ReadMemory", payload])
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
        .args(["tool", "ReadMemory", payload])
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
        .args(["tool", "WriteMemory", payload])
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
        .args(["tool", "WriteMemory", write_payload])
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "Created memory FILE: memory://move-extension-test",
        ));

    let move_payload = r#"{"source":"memory://move-extension-test","destination":"regression-test/Moved Extension Test"}"#;
    maenifold_cmd(&tmp)
        .args(["tool", "MoveMemory", move_payload])
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
        .args(["tool", "ReadMemory", read_payload])
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
        .args(["tool", "WriteMemory", write_payload])
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "Created memory FILE: memory://simple-move-test",
        ));

    let move_payload =
        r#"{"source":"memory://simple-move-test","destination":"Simple Move Renamed"}"#;
    maenifold_cmd(&tmp)
        .args(["tool", "MoveMemory", move_payload])
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "Moved memory FILE: memory://simple-move-test",
        ))
        .stdout(predicate::str::contains("memory://simple-move-renamed"));

    let read_payload = r#"{"identifier":"memory://simple-move-renamed","includeChecksum":true}"#;
    let read_output = maenifold_cmd(&tmp)
        .args(["tool", "ReadMemory", read_payload])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(read_output).expect("utf8");
    assert!(text.contains("Testing [[refactoring]] functionality."));
}
