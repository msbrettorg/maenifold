mod system_tools_cli_support;

// T-12.7: RTM NFR-1 - preserve directory/help CLI regression coverage after splitting the large system tools test file.
use predicates::prelude::*;
use serde_json::Value;
use system_tools_cli_support::*;

#[test]
// T-11.9: RTM FR-11.9 - JSON mode wraps output in envelope; markdown content lives in `data`
fn list_memories_json_mode_returns_envelope_with_markdown_data() {
    let tmp = isolated_root();

    let output = maenifold_cmd(&tmp)
        .args(["tool", "ListMemories", "{}", "--json"])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("valid JSON envelope");
    assert_eq!(json["success"], true, "success should be true");
    assert!(
        json["data"]
            .as_str()
            .expect("data is a string")
            .starts_with("# Directory: /"),
        "data should start with '# Directory: /'"
    );
    assert_eq!(json["error"], Value::Null, "error should be null");
}

#[test]
fn list_memories_nonexistent_path_returns_directory_not_found() {
    let tmp = isolated_root();

    maenifold_cmd(&tmp)
        .args([
            "tool",
            "ListMemories",
            r#"{"path":"non-existent-directory-xyz"}"#,
        ])
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "Directory not found: non-existent-directory-xyz",
        ));
}

#[test]
fn list_memories_includes_decay_metadata_and_top_level_folders() {
    let tmp = isolated_root();
    let root_payload = r#"{"title":"Seed Root","content":"Testing [[system-tools]]."}"#;
    maenifold_cmd(&tmp)
        .args(["tool", "WriteMemory", root_payload])
        .assert()
        .code(0);

    write_markdown_file(
        &tmp,
        "decay-metadata-tests/standard/created-test.md",
        "---\ntitle: Created Test\ntype: memory\nstatus: saved\ncreated: 2026-01-15T10:30:00Z\nmodified: 2026-01-15T10:30:00Z\n---\n\n# Created Test\n\nTest content for [[decay-metadata]].\n",
    );
    write_markdown_file(
        &tmp,
        "decay-metadata-tests/hidden/.ignored.md",
        "# Hidden\n\n[[hidden]].\n",
    );
    seed_last_accessed(
        &tmp,
        "memory://decay-metadata-tests/standard/created-test",
        "2026-01-20T14:45:00Z",
    );

    let parent_output = maenifold_cmd(&tmp)
        .args([
            "tool",
            "ListMemories",
            r#"{"path":"decay-metadata-tests"}"#,
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let parent_text = String::from_utf8(parent_output).expect("utf8");
    assert!(parent_text.contains("# Directory: decay-metadata-tests"));
    assert!(parent_text.contains("### Folders"));
    assert!(parent_text.contains("📁 standard/ (1 files)"));
    assert!(!parent_text.contains(".ignored.md"));

    let child_output = maenifold_cmd(&tmp)
        .args([
            "tool",
            "ListMemories",
            r#"{"path":"decay-metadata-tests/standard"}"#,
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let child_text = String::from_utf8(child_output).expect("utf8");
    assert!(child_text.contains("created: 2026-01-15"));
    assert!(child_text.contains("last_accessed: 2026-01-20"));
    assert!(child_text.contains("decay_weight:"));
}

#[test]
fn get_help_invalid_tool_lists_available_tools() {
    let tmp = isolated_root();

    let output = maenifold_cmd(&tmp)
        .args([
            "tool",
            "GetHelp",
            r#"{"toolName":"non-existent-tool-xyz"}"#,
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(output).expect("utf8");
    assert!(
        text.contains("No help file found for tool: non-existent-tool-xyz")
            || text.contains("not found")
    );
    assert!(text.contains("Available tools with help"));
    assert!(text.contains("writememory"));
}

#[test]
fn get_help_is_case_insensitive() {
    let tmp = isolated_root();

    let lower = maenifold_cmd(&tmp)
        .args([
            "tool",
            "GetHelp",
            r#"{"toolName":"writememory"}"#,
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let upper = maenifold_cmd(&tmp)
        .args([
            "tool",
            "GetHelp",
            r#"{"toolName":"WRITEMEMORY"}"#,
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let mixed = maenifold_cmd(&tmp)
        .args([
            "tool",
            "GetHelp",
            r#"{"toolName":"WriteMemory"}"#,
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let lower = String::from_utf8(lower).expect("utf8");
    let upper = String::from_utf8(upper).expect("utf8");
    let mixed = String::from_utf8(mixed).expect("utf8");

    assert_eq!(lower, upper);
    assert_eq!(upper, mixed);
}
