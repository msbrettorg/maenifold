// T-12.7: RTM NFR-1
//! Structured error coverage split from `cli_output_tests.rs`.

mod cli_output_support;

use cli_output_support::{
    assert_no_ansi_escape_sequences, isolated_root, json, run_json, write_memory_json,
};

#[test]
fn read_memory_not_found_has_structured_error() {
    // T-CLI-JSON-001: RTM FR-8.4
    let tmp = isolated_root();
    let value = run_json(
        &tmp,
        "ReadMemory",
        &json!({
            "identifier": "memory://non-existent-file-12345",
            "includeChecksum": true,
        }),
        1,
    );

    assert_eq!(value["success"], false);
    assert!(value["data"].is_null(), "data should be null: {value}");
    assert!(
        value["error"].is_object(),
        "error should be an object: {value}"
    );
    assert!(
        !value["error"]["code"]
            .as_str()
            .unwrap_or_default()
            .is_empty(),
        "error code should not be empty: {value}"
    );
    assert!(
        !value["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .is_empty(),
        "error message should not be empty: {value}"
    );
}

#[test]
fn sequential_thinking_validation_error_has_structured_error() {
    // T-CLI-JSON-001: RTM FR-8.4
    let tmp = isolated_root();
    let value = run_json(
        &tmp,
        "SequentialThinking",
        &json!({
            "response": "This response has no wiki links at all.",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2,
        }),
        1,
    );

    assert_eq!(value["success"], false);
    assert!(
        value["error"].is_object(),
        "error should be an object: {value}"
    );
    assert!(
        value["error"].get("code").is_some(),
        "missing error code: {value}"
    );
    let message = value["error"]["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("WikiLink"),
        "unexpected message: {message}"
    );
}

#[test]
fn delete_memory_without_confirmation_has_structured_error() {
    // T-CLI-JSON-001: RTM FR-8.4
    let tmp = isolated_root();
    let write_value =
        write_memory_json(&tmp, "Delete Test", "Content with [[delete-test]] concept.");
    let uri = write_value["data"]["uri"].as_str().expect("uri");

    let value = run_json(
        &tmp,
        "DeleteMemory",
        &json!({
            "identifier": uri,
            "confirm": false,
        }),
        1,
    );

    assert_eq!(value["success"], false);
    assert!(
        value["error"].is_object(),
        "error should be an object: {value}"
    );
    assert!(
        value["error"].get("code").is_some(),
        "missing error code: {value}"
    );
    assert!(
        value["error"].get("message").is_some(),
        "missing error message: {value}"
    );
}

#[test]
fn read_memory_error_has_no_ansi_escape_codes() {
    // T-CLI-JSON-001: RTM FR-8.2
    let tmp = isolated_root();
    let value = run_json(
        &tmp,
        "ReadMemory",
        &json!({
            "identifier": "memory://non-existent-ansi-test",
            "includeChecksum": true,
        }),
        1,
    );

    assert_no_ansi_escape_sequences(&serde_json::to_string(&value).expect("json text"));
}
