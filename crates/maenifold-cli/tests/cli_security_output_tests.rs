mod cli_security_support;

// T-12.0: RTM NFR-1 - preserve output-mode isolation and envelope security coverage after splitting cli_security_tests.rs.
use cli_security_support::*;
use serde_json::{Value, json};
use std::sync::Arc;
use std::thread;

#[test]
fn output_format_parallel_access_isolated_per_process() {
    let tmp = Arc::new(isolated_root());
    let mut handles = Vec::new();

    for index in 0..40 {
        let tmp = Arc::clone(&tmp);
        handles.push(thread::spawn(move || {
            if index % 2 == 0 {
                let output = run_tool_bytes(&tmp, "GetConfig", &json!({}), true, 0);
                let value: Value = serde_json::from_slice(&output).expect("json output");
                assert_eq!(value["success"], true, "unexpected output: {value}");
            } else {
                let output = run_tool_text(&tmp, "GetConfig", &json!({}), 0);
                assert!(
                    output.contains("Maenifold Configuration:"),
                    "unexpected markdown output: {output}"
                );
                assert!(
                    serde_json::from_str::<Value>(&output).is_err(),
                    "markdown mode should not return json: {output}"
                );
            }
        }));
    }

    for handle in handles {
        handle.join().expect("thread should finish cleanly");
    }
}

#[test]
fn output_format_switches_are_restored_across_invocations() {
    let tmp = isolated_root();

    let markdown_before = run_tool_text(&tmp, "GetConfig", &json!({}), 0);
    let json_output = run_tool_json(&tmp, "GetConfig", &json!({}), 0);
    let markdown_after = run_tool_text(&tmp, "GetConfig", &json!({}), 0);

    assert!(
        markdown_before.contains("Maenifold Configuration:"),
        "unexpected markdown output: {markdown_before}"
    );
    assert_eq!(
        json_output["success"], true,
        "unexpected json output: {json_output}"
    );
    assert!(
        markdown_after.contains("Maenifold Configuration:"),
        "unexpected markdown output after json call: {markdown_after}"
    );
}

#[test]
fn write_memory_json_output_contains_no_implementation_ansi_codes() {
    let tmp = isolated_root();
    let output = String::from_utf8(run_tool_bytes(
        &tmp,
        "WriteMemory",
        &json!({
            "title": "Clean JSON Test",
            "content": "Clean content with [[clean-json]]."
        }),
        true,
        0,
    ))
    .expect("utf8");

    assert!(
        !output.contains("\u{001b}["),
        "raw ansi escape leaked: {output}"
    );
    assert!(
        !output.contains("\\x1b["),
        "hex ansi escape leaked: {output}"
    );
}

#[test]
fn json_mode_produces_machine_parseable_output_while_markdown_mode_does_not() {
    let tmp = isolated_root();
    let json_output = run_tool_bytes(
        &tmp,
        "WriteMemory",
        &json!({
            "title": "Format Switch Test JSON",
            "content": "Content with [[format-test]]."
        }),
        true,
        0,
    );
    let markdown_output = run_tool_text(
        &tmp,
        "WriteMemory",
        &json!({
            "title": "Format Switch Test MD",
            "content": "Content with [[format-test]]."
        }),
        0,
    );

    serde_json::from_slice::<Value>(&json_output).expect("json mode should be parseable");
    assert!(
        serde_json::from_str::<Value>(&markdown_output).is_err(),
        "markdown mode should not be parseable json: {markdown_output}"
    );
}

#[test]
fn all_tools_json_output_uses_consistent_envelope_structure() {
    let tmp = isolated_root();
    let write_value = run_tool_json(
        &tmp,
        "WriteMemory",
        &json!({
            "title": "Consistency Test",
            "content": "Content with [[consistency-test]]."
        }),
        0,
    );
    let uri = write_value["data"]["uri"]
        .as_str()
        .expect("uri")
        .to_string();

    let write_again = run_tool_json(
        &tmp,
        "WriteMemory",
        &json!({
            "title": "New File",
            "content": "Content with [[new-file]]."
        }),
        0,
    );
    let read_value = run_tool_json(&tmp, "ReadMemory", &json!({ "identifier": uri }), 0);
    let search_value = run_tool_json(
        &tmp,
        "SearchMemories",
        &json!({ "query": "consistency" }),
        0,
    );
    let delete_without_confirm = run_tool_json(
        &tmp,
        "DeleteMemory",
        &json!({ "identifier": "memory://new-file", "confirm": false }),
        1,
    );
    let read_not_found = run_tool_json(
        &tmp,
        "ReadMemory",
        &json!({ "identifier": "memory://does-not-exist" }),
        1,
    );

    for response in [
        write_value,
        write_again,
        read_value,
        search_value,
        delete_without_confirm,
        read_not_found,
    ] {
        assert_envelope_shape(&response);
    }
}

// T-12.7: RTM NFR-1 - preserve JSON payload escaping coverage while consolidating split CLI security tests.
#[test]
fn write_memory_content_with_braces_produces_valid_json_without_root_injection() {
    let tmp = isolated_root();
    let value = run_tool_json(
        &tmp,
        "WriteMemory",
        &json!({
            "title": "Brace Injection Test",
            "content": "Content with [[brace-test]] and }, \"injected\": true}"
        }),
        0,
    );

    assert_eq!(value["success"], true, "unexpected output: {value}");
    assert!(
        value.get("injected").is_none(),
        "unexpected root injection: {value}"
    );
}
