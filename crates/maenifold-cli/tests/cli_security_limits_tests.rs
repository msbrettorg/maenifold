mod cli_security_support;

// T-12.0: RTM NFR-1 - preserve size/depth/validation security coverage after splitting cli_security_tests.rs.
use cli_security_support::*;
use serde_json::json;

#[test]
fn write_memory_many_wikilinks_succeeds() {
    let tmp = isolated_root();
    let content = (0..100)
        .map(|index| format!("[[concept-{index}]]"))
        .collect::<Vec<_>>()
        .join(" ");

    let value = run_tool_json(
        &tmp,
        "WriteMemory",
        &json!({
            "title": "Many Links Test",
            "content": content
        }),
        0,
    );

    assert_eq!(value["success"], true, "unexpected output: {value}");
}

#[test]
fn cli_json_payload_max_depth_is_enforced() {
    let tmp = isolated_root();
    let output = run_tool_json(
        &tmp,
        "WriteMemory",
        &json!({ "title": create_deep_object(33) }),
        1,
    );

    assert_eq!(output["success"], false, "unexpected output: {output}");
    assert_eq!(output["error"]["code"], "INVALID_PAYLOAD");
    assert!(
        output["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .contains("depth"),
        "unexpected error message: {output}"
    );
}

#[test]
fn write_memory_empty_content_produces_valid_error_json() {
    let tmp = isolated_root();
    let value = run_tool_json(
        &tmp,
        "WriteMemory",
        &json!({
            "title": "Empty Test",
            "content": ""
        }),
        1,
    );

    assert_eq!(value["success"], false, "unexpected output: {value}");
}

#[test]
fn write_memory_very_long_title_returns_structured_path_too_long_error() {
    let tmp = isolated_root();
    let value = run_tool_json(
        &tmp,
        "WriteMemory",
        &json!({
            "title": "A".repeat(500),
            "content": "Content with [[long-title-test]]."
        }),
        1,
    );

    assert_eq!(value["success"], false, "unexpected output: {value}");
    assert_eq!(value["error"]["code"], "PATH_TOO_LONG");
    let message = value["error"]["message"].as_str().unwrap_or_default();
    assert!(!message.contains("/Users/"), "unix path leaked: {message}");
    assert!(
        !message.contains("\\Users\\"),
        "windows path leaked: {message}"
    );
}
