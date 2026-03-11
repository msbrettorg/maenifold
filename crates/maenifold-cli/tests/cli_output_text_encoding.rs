// T-12.7: RTM NFR-1
//! Markdown rendering, ANSI, and UTF-8 coverage split from `cli_output_tests.rs`.

mod cli_output_support;

use cli_output_support::{
    Value, assert_no_ansi_escape_sequences, isolated_root, json, run_json, run_text,
    write_memory_json,
};

#[test]
fn write_memory_without_json_flag_returns_markdown() {
    // T-CLI-JSON-001: RTM FR-8.5
    let tmp = isolated_root();
    let text = run_text(
        &tmp,
        "WriteMemory",
        &json!({
            "title": "Markdown Output Test",
            "content": "Content with [[markdown-output]] concept.",
            "folder": "cli-json-tests",
        }),
        false,
        0,
    );

    assert!(
        text.starts_with("Created memory FILE:"),
        "unexpected output: {text}"
    );
    assert!(
        serde_json::from_str::<Value>(&text).is_err(),
        "markdown output should not parse as JSON: {text}"
    );
}

#[test]
fn search_memories_without_json_flag_returns_markdown() {
    // T-CLI-JSON-001: RTM FR-8.5
    let tmp = isolated_root();
    write_memory_json(&tmp, "Markdown Search", "Content with [[markdown-search]].");

    let text = run_text(
        &tmp,
        "SearchMemories",
        &json!({
            "query": "markdown-search",
            "mode": "Hybrid",
            "pageSize": 10,
            "page": 1,
        }),
        false,
        0,
    );

    // T-13.7: RTM FR-21 — updated assertions for C#-parity emoji markdown format
    assert!(
        text.contains("🔍 **Hybrid Search**"),
        "unexpected output: {text}"
    );
    assert!(
        text.contains("matches (page 1):"),
        "unexpected output: {text}"
    );
    assert!(
        text.contains("📄 **Markdown Search** (memory://cli-json-tests/markdown-search)"),
        "unexpected output: {text}"
    );
    assert!(
        serde_json::from_str::<Value>(&text).is_err(),
        "markdown output should not parse as JSON: {text}"
    );
}

#[test]
fn write_memory_json_output_has_no_ansi_escape_codes() {
    // T-CLI-JSON-001: RTM FR-8.2
    let tmp = isolated_root();
    let value = run_json(
        &tmp,
        "WriteMemory",
        &json!({
            "title": "ANSI Test",
            "content": "Content with [[ansi-test]] concept.",
            "folder": "cli-json-tests",
        }),
        0,
    );

    assert_no_ansi_escape_sequences(&serde_json::to_string(&value).expect("json text"));
}
