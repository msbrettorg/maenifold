use assert_cmd::Command;
use serde_json::{Value, json};
use tempfile::TempDir;

const TEST_FOLDER: &str = "cli-json-tests";

fn isolated_root() -> TempDir {
    TempDir::new().expect("tmp")
}

fn maenifold_cmd(tmp: &TempDir) -> Command {
    // T-CLI-JSON-001: RTM FR-8.1
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("maenifold"));
    cmd.env("MAENIFOLD_ROOT", tmp.path())
        .env("MAENIFOLD_DATABASE_PATH", tmp.path().join("memory.db"));
    cmd
}

fn run_output(
    tmp: &TempDir,
    tool: &str,
    payload: &Value,
    json_mode: bool,
    expected_code: i32,
) -> Vec<u8> {
    let mut cmd = maenifold_cmd(tmp);
    cmd.arg("--tool")
        .arg(tool)
        .arg("--payload")
        .arg(payload.to_string());
    if json_mode {
        cmd.arg("--json");
    }
    cmd.assert().code(expected_code).get_output().stdout.clone()
}

fn run_json(tmp: &TempDir, tool: &str, payload: &Value, expected_code: i32) -> Value {
    let output = run_output(tmp, tool, payload, true, expected_code);
    serde_json::from_slice(&output).expect("valid json")
}

fn run_text(
    tmp: &TempDir,
    tool: &str,
    payload: &Value,
    json_mode: bool,
    expected_code: i32,
) -> String {
    String::from_utf8(run_output(tmp, tool, payload, json_mode, expected_code)).expect("utf8")
}

fn write_memory_json(tmp: &TempDir, title: &str, content: &str) -> Value {
    // T-CLI-JSON-001: RTM FR-8.2
    run_json(
        tmp,
        "WriteMemory",
        &json!({
            "title": title,
            "content": content,
            "folder": TEST_FOLDER,
        }),
        0,
    )
}

fn sync_memory_graph(tmp: &TempDir) {
    let _ = run_text(tmp, "Sync", &json!({}), true, 0);
}

fn assert_no_ansi_escape_sequences(text: &str) {
    // T-CLI-JSON-001: RTM FR-8.2
    assert!(
        !text.contains("\u{1b}[") && !text.contains("\x1b["),
        "output should not contain ANSI escape codes: {text}"
    );
}

#[test]
fn write_memory_with_json_flag_produces_valid_json() {
    // T-CLI-JSON-001: RTM FR-8.1
    let tmp = isolated_root();
    let output = run_output(
        &tmp,
        "WriteMemory",
        &json!({
            "title": "JSON Output Test",
            "content": "Test content with [[json-output]] concept.",
            "folder": TEST_FOLDER,
        }),
        true,
        0,
    );

    serde_json::from_slice::<Value>(&output).expect("WriteMemory JSON should parse");
}

#[test]
fn sequential_thinking_with_json_flag_produces_valid_json() {
    // T-CLI-JSON-001: RTM FR-8.1
    let tmp = isolated_root();
    let output = run_output(
        &tmp,
        "SequentialThinking",
        &json!({
            "response": "Test thought with [[sequential-thinking]] concept.",
            "nextThoughtNeeded": false,
            "thoughtNumber": 0,
            "totalThoughts": 1,
            "conclusion": "Test conclusion with [[test-concept]].",
        }),
        true,
        0,
    );

    serde_json::from_slice::<Value>(&output).expect("SequentialThinking JSON should parse");
}

#[test]
fn search_memories_with_json_flag_produces_valid_json() {
    // T-CLI-JSON-001: RTM FR-8.2
    let tmp = isolated_root();
    write_memory_json(&tmp, "Search Test", "Content about [[testing]] concepts.");

    let output = run_output(
        &tmp,
        "SearchMemories",
        &json!({
            "query": "testing",
            "mode": "Hybrid",
            "pageSize": 10,
            "page": 1,
        }),
        true,
        0,
    );

    serde_json::from_slice::<Value>(&output).expect("SearchMemories JSON should parse");
}

#[test]
fn read_memory_with_json_flag_produces_valid_json() {
    // T-CLI-JSON-001: RTM FR-8.2
    let tmp = isolated_root();
    let write_value = write_memory_json(&tmp, "Read Test", "Content with [[read-memory]] concept.");
    let uri = write_value["data"]["uri"].as_str().expect("uri");

    let output = run_output(
        &tmp,
        "ReadMemory",
        &json!({
            "identifier": uri,
            "includeChecksum": true,
        }),
        true,
        0,
    );

    serde_json::from_slice::<Value>(&output).expect("ReadMemory JSON should parse");
}

#[test]
fn build_context_with_json_flag_produces_valid_json() {
    // T-CLI-JSON-001: RTM FR-8.2
    let tmp = isolated_root();
    write_memory_json(
        &tmp,
        "Context Test",
        "Content about [[graph-context]] and [[relationships]].",
    );
    sync_memory_graph(&tmp);

    let output = run_output(
        &tmp,
        "BuildContext",
        &json!({
            "conceptName": "graph-context",
            "depth": 1,
            "maxEntities": 10,
        }),
        true,
        0,
    );

    let value: Value = serde_json::from_slice(&output).expect("BuildContext JSON should parse");
    assert_eq!(value["success"], true);
}

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
            "folder": TEST_FOLDER,
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
fn write_memory_without_json_flag_returns_markdown() {
    // T-CLI-JSON-001: RTM FR-8.5
    let tmp = isolated_root();
    let text = run_text(
        &tmp,
        "WriteMemory",
        &json!({
            "title": "Markdown Output Test",
            "content": "Content with [[markdown-output]] concept.",
            "folder": TEST_FOLDER,
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

    assert!(
        text.contains("# Search Results"),
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
            "folder": TEST_FOLDER,
        }),
        0,
    );

    assert_no_ansi_escape_sequences(&serde_json::to_string(&value).expect("json text"));
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

#[test]
fn write_memory_json_output_is_valid_utf8() {
    // T-CLI-JSON-001: RTM FR-8.2
    let tmp = isolated_root();
    let output = run_output(
        &tmp,
        "WriteMemory",
        &json!({
            "title": "UTF-8 Test",
            "content": "Content with [[unicode]] chars: café, ∑, 😀.",
            "folder": TEST_FOLDER,
        }),
        true,
        0,
    );

    let text = String::from_utf8(output).expect("utf8 output");
    let round_trip = String::from_utf8(text.as_bytes().to_vec()).expect("round trip utf8");
    assert_eq!(round_trip, text);
    serde_json::from_str::<Value>(&text).expect("UTF-8 JSON should parse");
}

#[test]
fn search_memories_json_with_special_chars_is_valid_utf8() {
    // T-CLI-JSON-001: RTM FR-8.2
    let tmp = isolated_root();
    write_memory_json(
        &tmp,
        "Special Chars Test",
        "Content with [[special-chars]]: quotes \"test\", backslash \\, newline \\n, tab \\t.",
    );

    let output = run_output(
        &tmp,
        "SearchMemories",
        &json!({
            "query": "special-chars",
            "mode": "Hybrid",
            "pageSize": 10,
            "page": 1,
        }),
        true,
        0,
    );

    let text = String::from_utf8(output).expect("utf8 output");
    let round_trip = String::from_utf8(text.as_bytes().to_vec()).expect("round trip utf8");
    assert_eq!(round_trip, text);
    serde_json::from_str::<Value>(&text).expect("special-char JSON should parse");
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
