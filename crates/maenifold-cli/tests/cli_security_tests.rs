use assert_cmd::Command;
use serde_json::{Value, json};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

fn isolated_root() -> TempDir {
    TempDir::new().expect("tmp")
}

fn maenifold_cmd(tmp: &TempDir) -> Command {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("maenifold"));
    cmd.env("MAENIFOLD_ROOT", tmp.path())
        .env("MAENIFOLD_DATABASE_PATH", tmp.path().join("memory.db"));
    cmd
}

// T-0.1: RTM NFR-3 - port the C# CLI JSON security regressions so adversarial inputs keep the Rust CLI on a valid, machine-readable contract.
fn run_tool_bytes(
    tmp: &TempDir,
    tool: &str,
    payload: &Value,
    json_mode: bool,
    expected_code: i32,
) -> Vec<u8> {
    let payload = serde_json::to_string(payload).expect("payload json");
    let mut cmd = maenifold_cmd(tmp);
    let mut args = vec![
        "--tool".to_string(),
        tool.to_string(),
        "--payload".to_string(),
        payload,
    ];
    if json_mode {
        args.push("--json".to_string());
    }

    cmd.args(&args)
        .assert()
        .code(expected_code)
        .get_output()
        .stdout
        .clone()
}

fn run_tool_json(tmp: &TempDir, tool: &str, payload: &Value, expected_code: i32) -> Value {
    serde_json::from_slice(&run_tool_bytes(tmp, tool, payload, true, expected_code))
        .expect("valid json")
}

fn run_tool_text(tmp: &TempDir, tool: &str, payload: &Value, expected_code: i32) -> String {
    String::from_utf8(run_tool_bytes(tmp, tool, payload, false, expected_code)).expect("utf8")
}

fn create_sequential_session(tmp: &TempDir) -> String {
    let value = run_tool_json(
        tmp,
        "SequentialThinking",
        &json!({
            "response": "Starting [[cli-json-security]] session.",
            "thoughtNumber": 0,
            "totalThoughts": 2,
            "nextThoughtNeeded": true
        }),
        0,
    );
    value["data"]["sessionId"]
        .as_str()
        .expect("sessionId")
        .to_string()
}

fn assert_envelope_shape(value: &Value) {
    assert!(value.get("success").is_some(), "missing success: {value}");
    assert!(value.get("data").is_some(), "missing data: {value}");
    assert!(value.get("error").is_some(), "missing error: {value}");
}

fn create_deep_object(depth: usize) -> Value {
    let mut value = json!("leaf");
    for _ in 0..depth {
        value = json!({ "nested": value });
    }
    value
}

#[test]
fn write_memory_content_with_double_quotes_produces_valid_json() {
    let tmp = isolated_root();
    let value = run_tool_json(
        &tmp,
        "WriteMemory",
        &json!({
            "title": "Quote Injection Test",
            "content": "Content with [[injection]] and \"malicious\": \"payload\" here."
        }),
        0,
    );

    assert_eq!(value["success"], true, "unexpected output: {value}");
}

#[test]
fn write_memory_content_with_backslashes_produces_valid_json() {
    let tmp = isolated_root();
    let value = run_tool_json(
        &tmp,
        "WriteMemory",
        &json!({
            "title": "Backslash Injection Test",
            "content": r"Content with [[backslash]] and paths like C:\Users\test\file.txt"
        }),
        0,
    );

    assert_eq!(value["success"], true, "unexpected output: {value}");
}

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

#[test]
fn write_memory_content_with_newlines_produces_valid_json() {
    let tmp = isolated_root();
    let value = run_tool_json(
        &tmp,
        "WriteMemory",
        &json!({
            "title": "Newline Injection Test",
            "content": "Content with [[newline]]\nand multiple\r\nline breaks\rhere."
        }),
        0,
    );

    assert_eq!(value["success"], true, "unexpected output: {value}");
}

#[test]
fn write_memory_wikilink_with_special_chars_produces_valid_json() {
    let tmp = isolated_root();
    let payload = serde_json::to_string(&json!({
        "title": "WikiLink Injection Test",
        "content": "Testing [[concept\"injection]] and [[another}concept]] here."
    }))
    .expect("payload json");
    let output = maenifold_cmd(&tmp)
        .args(["--tool", "WriteMemory", "--payload", &payload, "--json"])
        .output()
        .expect("run maenifold");
    assert!(
        matches!(output.status.code(), Some(0 | 1)),
        "unexpected exit status: {:?}",
        output.status.code()
    );

    let value: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_envelope_shape(&value);
}

#[test]
fn write_memory_content_with_tabs_produces_valid_json() {
    let tmp = isolated_root();
    let value = run_tool_json(
        &tmp,
        "WriteMemory",
        &json!({
            "title": "Tab Injection Test",
            "content": "Content with [[tabs]]\there\tand\tthere."
        }),
        0,
    );

    assert_eq!(value["success"], true, "unexpected output: {value}");
}

#[test]
fn write_memory_content_with_bom_produces_valid_json() {
    let tmp = isolated_root();
    let value = run_tool_json(
        &tmp,
        "WriteMemory",
        &json!({
            "title": "BOM Injection Test",
            "content": format!("\u{feff}Content with [[bom-test]] after BOM.")
        }),
        0,
    );

    assert_eq!(value["success"], true, "unexpected output: {value}");
}

#[test]
fn write_memory_content_with_null_bytes_produces_valid_json() {
    let tmp = isolated_root();
    let value = run_tool_json(
        &tmp,
        "WriteMemory",
        &json!({
            "title": "Null Byte Test",
            "content": "Content with [[null-byte]] and \0 null byte here."
        }),
        0,
    );

    assert_eq!(value["success"], true, "unexpected output: {value}");
}

#[test]
fn write_memory_content_with_control_chars_produces_valid_json() {
    let tmp = isolated_root();
    let mut content = String::from("Content with [[control-chars]]");
    for code in 0u8..32 {
        if code != b'\n' && code != b'\r' {
            content.push(char::from(code));
        }
    }
    content.push_str(" end.");

    let value = run_tool_json(
        &tmp,
        "WriteMemory",
        &json!({
            "title": "Control Char Test",
            "content": content
        }),
        0,
    );

    assert_eq!(value["success"], true, "unexpected output: {value}");
}

#[test]
fn write_memory_content_with_homoglyphs_produces_valid_json() {
    let tmp = isolated_root();
    let value = run_tool_json(
        &tmp,
        "WriteMemory",
        &json!({
            "title": "Homoglyph Test",
            "content": "Content with [[homoglyph]] and ： ｛ ｝ chars."
        }),
        0,
    );

    assert_eq!(value["success"], true, "unexpected output: {value}");
}

#[test]
fn read_memory_response_is_valid_utf8() {
    let tmp = isolated_root();
    let written = run_tool_json(
        &tmp,
        "WriteMemory",
        &json!({
            "title": "UTF8 Validation",
            "content": "Content with [[utf8-test]]."
        }),
        0,
    );
    let uri = written["data"]["uri"].as_str().expect("uri");

    let output = run_tool_bytes(
        &tmp,
        "ReadMemory",
        &json!({ "identifier": uri, "includeChecksum": true }),
        true,
        0,
    );
    let text = String::from_utf8(output.clone()).expect("utf8");

    assert_eq!(
        text.as_bytes(),
        output.as_slice(),
        "output must round-trip as utf8"
    );
}

#[test]
fn write_memory_large_content_is_handled_gracefully() {
    let tmp = isolated_root();
    let large_content = format!("Content with [[large-payload]] {}", "A".repeat(100 * 1024));

    let value = run_tool_json(
        &tmp,
        "WriteMemory",
        &json!({
            "title": "Large Content Test",
            "content": large_content
        }),
        0,
    );

    assert_eq!(value["success"], true, "unexpected output: {value}");
}

#[test]
fn write_memory_many_wikilinks_is_handled_efficiently() {
    let tmp = isolated_root();
    let content = (0..100)
        .map(|index| format!("[[concept-{index}]]"))
        .collect::<Vec<_>>()
        .join(" ");

    let start = Instant::now();
    let value = run_tool_json(
        &tmp,
        "WriteMemory",
        &json!({
            "title": "Many Links Test",
            "content": content
        }),
        0,
    );
    let elapsed = start.elapsed();

    assert_eq!(value["success"], true, "unexpected output: {value}");
    assert!(
        elapsed < Duration::from_secs(5),
        "processing many wikilinks took too long: {elapsed:?}"
    );
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
fn write_memory_content_with_ansi_codes_produces_valid_json() {
    let tmp = isolated_root();
    let value = run_tool_json(
        &tmp,
        "WriteMemory",
        &json!({
            "title": "ANSI Escape Test",
            "content": "Content with [[ansi-test]] and \u{001b}[31mred text\u{001b}[0m here."
        }),
        0,
    );

    assert_eq!(value["success"], true, "unexpected output: {value}");
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

#[test]
fn write_memory_unicode_title_produces_valid_json() {
    let tmp = isolated_root();
    let value = run_tool_json(
        &tmp,
        "WriteMemory",
        &json!({
            "title": "Test with emojis and unicode 🚀✨",
            "content": "Content with [[unicode-title]]."
        }),
        0,
    );

    assert_eq!(value["success"], true, "unexpected output: {value}");
}

#[test]
fn write_memory_null_folder_produces_valid_json() {
    let tmp = isolated_root();
    let value = run_tool_json(
        &tmp,
        "WriteMemory",
        &json!({
            "title": "Null Folder Test",
            "content": "Content with [[null-folder-test]].",
            "folder": Value::Null
        }),
        0,
    );

    assert_eq!(value["success"], true, "unexpected output: {value}");
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
