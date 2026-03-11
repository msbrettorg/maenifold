// T-12.7: RTM NFR-1
//! JSON envelope and data-field coverage split from `cli_output_tests.rs`.

mod cli_output_support;

use cli_output_support::{isolated_root, json, run_json, write_memory_json};

#[test]
fn write_memory_json_success_has_required_fields() {
    // T-CLI-JSON-001: RTM FR-8.3
    let tmp = isolated_root();
    let value = run_json(
        &tmp,
        "WriteMemory",
        &json!({
            "title": "Required Fields Test",
            "content": "Content with [[required-fields]] validation.",
            "folder": "cli-json-tests",
        }),
        0,
    );

    assert_eq!(value["success"], true);
    assert!(value.get("data").is_some(), "missing data field: {value}");
    assert!(
        value["data"].is_object(),
        "data should be an object: {value}"
    );
    assert!(value.get("error").is_some(), "missing error field: {value}");
    assert!(value["error"].is_null(), "error should be null: {value}");
}

#[test]
fn search_memories_json_success_has_required_fields() {
    // T-CLI-JSON-001: RTM FR-8.3
    let tmp = isolated_root();
    write_memory_json(
        &tmp,
        "Fields Test",
        "Content for [[field-validation]] test.",
    );

    let value = run_json(
        &tmp,
        "SearchMemories",
        &json!({
            "query": "field-validation",
            "mode": "Hybrid",
            "pageSize": 10,
            "page": 1,
        }),
        0,
    );

    assert!(
        value.get("success").is_some(),
        "missing success field: {value}"
    );
    assert!(value.get("data").is_some(), "missing data field: {value}");
    assert!(value.get("error").is_some(), "missing error field: {value}");
}

#[test]
fn sequential_thinking_json_success_has_required_fields() {
    // T-CLI-JSON-001: RTM FR-8.3
    let tmp = isolated_root();
    let value = run_json(
        &tmp,
        "SequentialThinking",
        &json!({
            "response": "Thought with [[required-fields-test]].",
            "nextThoughtNeeded": false,
            "thoughtNumber": 0,
            "totalThoughts": 1,
            "conclusion": "Conclusion with [[test-complete]].",
        }),
        0,
    );

    assert_eq!(value["success"], true);
    assert!(
        value["data"].is_object(),
        "data should be an object: {value}"
    );
    assert!(value.get("error").is_some(), "missing error field: {value}");
    assert!(
        value["data"].get("sessionId").is_some(),
        "missing sessionId: {value}"
    );
}

#[test]
fn read_memory_json_success_data_includes_content() {
    // T-CLI-JSON-001: RTM FR-8.3
    let tmp = isolated_root();
    let write_value = write_memory_json(
        &tmp,
        "Content Data Test",
        "File content with [[data-test]] concept.",
    );
    let uri = write_value["data"]["uri"].as_str().expect("uri");

    let value = run_json(
        &tmp,
        "ReadMemory",
        &json!({
            "identifier": uri,
            "includeChecksum": true,
        }),
        0,
    );

    let data = &value["data"];
    assert!(
        data.get("content").is_some(),
        "missing content field: {value}"
    );
    assert!(
        data["content"]
            .as_str()
            .unwrap_or_default()
            .contains("[[data-test]]"),
        "content should preserve wikilinks: {value}"
    );
    assert!(data.get("uri").is_some(), "missing uri field: {value}");
}

#[test]
fn search_memories_json_success_data_includes_pagination() {
    // T-CLI-JSON-001: RTM FR-8.3
    let tmp = isolated_root();
    for index in 0..5 {
        let title = format!("Pagination Test {index}");
        let content = format!("Content with [[pagination-test]] number {index}.");
        write_memory_json(&tmp, &title, &content);
    }

    let value = run_json(
        &tmp,
        "SearchMemories",
        &json!({
            "query": "pagination-test",
            "mode": "Hybrid",
            "pageSize": 2,
            "page": 1,
        }),
        0,
    );

    let data = &value["data"];
    assert!(
        data.get("results").is_some(),
        "missing results field: {value}"
    );
    assert!(
        data.get("totalCount").is_some(),
        "missing totalCount field: {value}"
    );
    assert!(data.get("page").is_some(), "missing page field: {value}");
    assert!(
        data.get("pageSize").is_some(),
        "missing pageSize field: {value}"
    );
}

#[test]
fn sequential_thinking_json_success_data_includes_session_info() {
    // T-CLI-JSON-001: RTM FR-8.3
    let tmp = isolated_root();
    let value = run_json(
        &tmp,
        "SequentialThinking",
        &json!({
            "response": "Creating session with [[session-info-test]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 3,
        }),
        0,
    );

    let data = &value["data"];
    assert!(
        data["sessionId"]
            .as_str()
            .unwrap_or_default()
            .starts_with("session-"),
        "unexpected session id: {value}"
    );
    assert!(
        data.get("thoughtNumber").is_some(),
        "missing thoughtNumber: {value}"
    );
    assert!(
        data.get("totalThoughts").is_some(),
        "missing totalThoughts: {value}"
    );
    assert!(data.get("status").is_some(), "missing status: {value}");
}
