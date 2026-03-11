mod cli_security_support;

// T-12.0: RTM NFR-1 - preserve sanitized error/envelope CLI security coverage after splitting cli_security_tests.rs.
use cli_security_support::*;
use serde_json::json;

#[test]
fn read_memory_error_does_not_leak_stack_trace_details() {
    let tmp = isolated_root();
    let output = run_tool_bytes(
        &tmp,
        "ReadMemory",
        &json!({ "identifier": "memory://non-existent-file-12345" }),
        true,
        1,
    );
    let text = String::from_utf8(output).expect("utf8");

    assert!(!text.contains("   at "), "stack trace leaked: {text}");
    assert!(
        !text.contains("StackTrace"),
        "stack trace field leaked: {text}"
    );
    assert!(!text.contains(".cs:line"), "source file leaked: {text}");
}

#[test]
fn read_memory_error_does_not_expose_full_paths() {
    let tmp = isolated_root();
    let value = run_tool_json(
        &tmp,
        "ReadMemory",
        &json!({ "identifier": "memory://non-existent-file" }),
        1,
    );
    let message = value["error"]["message"].as_str().unwrap_or_default();

    assert!(
        !message.starts_with("C:\\"),
        "windows path leaked: {message}"
    );
    assert!(
        !message.starts_with("/Users/")
            && !message.starts_with("/home/")
            && !message.starts_with("/var/")
            && !message.starts_with("/etc/"),
        "unix path leaked: {message}"
    );
}

#[test]
fn read_memory_error_uses_application_defined_error_codes() {
    let tmp = isolated_root();
    let value = run_tool_json(
        &tmp,
        "ReadMemory",
        &json!({ "identifier": "memory://non-existent-file" }),
        1,
    );
    let code = value["error"]["code"].as_str().unwrap_or_default();

    assert!(!code.contains("Exception"), "exception type leaked: {code}");
    assert!(!code.contains("System."), "namespace leaked: {code}");
}

#[test]
fn search_memories_no_results_produces_valid_json() {
    let tmp = isolated_root();
    let value = run_tool_json(
        &tmp,
        "SearchMemories",
        &json!({
            "query": "xyznonexistentquery12345",
            "mode": "Hybrid"
        }),
        0,
    );

    assert_eq!(value["success"], true, "unexpected output: {value}");
    assert_eq!(value["data"]["totalCount"], 0);
    assert_eq!(
        value["data"]["results"].as_array().map(Vec::len),
        Some(0),
        "unexpected results: {value}"
    );
}

#[test]
fn sequential_thinking_cancel_produces_valid_json() {
    let tmp = isolated_root();
    let session_id = create_sequential_session(&tmp);

    let value = run_tool_json(
        &tmp,
        "SequentialThinking",
        &json!({
            "sessionId": session_id,
            "cancel": true,
            "thoughtNumber": 1
        }),
        0,
    );

    assert_eq!(value["success"], true, "unexpected output: {value}");
    assert_eq!(value["data"]["status"], "cancelled");
}
