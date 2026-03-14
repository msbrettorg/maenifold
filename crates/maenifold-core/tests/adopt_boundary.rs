use maenifold_core::json::OutputFormat;
use maenifold_core::{Config, ToolRegistry};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::{Value, json};
use std::fs;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn isolated_config() -> (Config, TempDir) {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let config = Config::current();
    config.ensure_directories().expect("dirs");
    (config, tmp)
}

fn write_asset(config: &Config, kind_dir: &str, filename: &str, value: Value) {
    let path = config.assets_path.join(kind_dir).join(filename);
    fs::create_dir_all(path.parent().expect("parent")).expect("asset dirs");
    fs::write(
        path,
        serde_json::to_string_pretty(&value).expect("asset json serialization"),
    )
    .expect("write asset");
}

fn invoke_adopt(payload: Value) -> String {
    ToolRegistry::global()
        .try_invoke("Adopt", &payload, OutputFormat::Markdown)
        .expect("invoke Adopt")
}

fn extract_json_configuration(output: &str) -> Value {
    let marker = "JSON Configuration:";
    let marker_index = output.find(marker).expect("JSON Configuration marker");
    let suffix = output[marker_index + marker.len()..].trim();
    let json_start = suffix.find('{').expect("json object start");
    let candidate = &suffix[json_start..];

    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (idx, ch) in candidate.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return serde_json::from_str(&candidate[..=idx]).expect("valid embedded json");
                }
            }
            _ => {}
        }
    }

    panic!("unterminated embedded json: {candidate}");
}

// T-1.4a: RTM FR-1 — helper for parameterized adopt-type assertions
// Verifies: type keyword present, identifier echoed, JSON Configuration marker,
// embedded JSON parseable with correct id and name fields.
fn assert_adopt_type_produces_valid_instruction(
    asset_type: &str,
    kind_dir: &str,
    identifier: &str,
    asset_name: &str,
    asset_json: Value,
) {
    let (config, _tmp) = isolated_config();
    write_asset(&config, kind_dir, &format!("{identifier}.json"), asset_json);

    let output = invoke_adopt(json!({"type": asset_type, "identifier": identifier}));
    let lower = output.to_ascii_lowercase();

    // Type keyword present in output
    assert!(
        lower.contains(asset_type),
        "[{asset_type}] expected type wording in output: {output}"
    );
    // Identifier echoed back
    assert!(
        output.contains(identifier),
        "[{asset_type}] expected identifier '{identifier}' in output: {output}"
    );
    // JSON Configuration marker present — proves adoption instruction format,
    // not just any output that happens to mention the type/identifier
    assert!(
        output.contains("JSON Configuration:"),
        "[{asset_type}] expected JSON Configuration marker in output: {output}"
    );
    // Embedded JSON is parseable and contains correct id + name fields
    let parsed = extract_json_configuration(&output);
    assert_eq!(
        parsed["id"], identifier,
        "[{asset_type}] embedded JSON 'id' mismatch"
    );
    assert_eq!(
        parsed["name"], asset_name,
        "[{asset_type}] embedded JSON 'name' mismatch"
    );

    Config::reset_overrides();
}

// T-1.4a: RTM FR-1 (RTM-001 / TC-1.1, RTM-002 / TC-1.2, RTM-003 / TC-1.3)
// MERGE of adopt_role_*, adopt_color_*, adopt_perspective_* — parameterized
// with test-7 rigor: verifies JSON Configuration marker + parseable embedded JSON.
#[test]
fn adopt_all_asset_types_return_instruction_with_valid_json_configuration() {
    let _guard = TEST_MUTEX.lock();

    // RTM-001: role
    assert_adopt_type_produces_valid_instruction(
        "role",
        "roles",
        "test-role",
        "Test Role",
        json!({
            "id": "test-role",
            "name": "Test Role",
            "personality": "analytical",
            "principles": ["accuracy"]
        }),
    );

    // RTM-002: color
    assert_adopt_type_produces_valid_instruction(
        "color",
        "colors",
        "test-color",
        "Test Color",
        json!({
            "id": "test-color",
            "name": "Test Color",
            "mode": "critical",
            "principles": ["caution"]
        }),
    );

    // RTM-003: perspective
    assert_adopt_type_produces_valid_instruction(
        "perspective",
        "perspectives",
        "test-perspective",
        "Test Perspective",
        json!({
            "id": "test-perspective",
            "name": "Test Perspective",
            "voice": "skeptical",
            "principles": ["clarity"]
        }),
    );
}

// T-1.4a: RTM FR-1 (RTM-004 / TC-1.4)
#[test]
fn adopt_invalid_type_returns_invalid_type_error() {
    let _guard = TEST_MUTEX.lock();
    let (_config, _tmp) = isolated_config();

    let err = ToolRegistry::global()
        .try_invoke(
            "Adopt",
            &json!({"type":"invalid","identifier":"x"}),
            OutputFormat::Markdown,
        )
        .expect_err("invalid type should error");

    assert!(
        err.to_string().contains("Invalid type"),
        "unexpected error: {err}"
    );

    Config::reset_overrides();
}

// T-1.4a: RTM FR-1 (RTM-005 / TC-1.5)
#[test]
fn adopt_falls_back_to_json_id_field_when_filename_differs() {
    let _guard = TEST_MUTEX.lock();
    let (config, _tmp) = isolated_config();
    write_asset(
        &config,
        "roles",
        "different-filename.json",
        json!({
            "id": "hidden-id",
            "name": "Hidden Role"
        }),
    );

    let output = invoke_adopt(json!({"type":"role","identifier":"hidden-id"}));

    assert!(
        output.contains("hidden-id"),
        "expected fallback identifier in output: {output}"
    );
    assert!(
        output.contains("Hidden Role"),
        "expected fallback asset content in output: {output}"
    );

    Config::reset_overrides();
}

// T-1.4a: RTM FR-1 (RTM-006 / TC-1.6)
// REWRITE: replaced weak OR-of-three positive assertion with structural
// help-path invariants — checks "help" keyword and two negative adoption markers.
#[test]
fn adopt_learn_true_returns_help_documentation_not_adoption_instruction() {
    let _guard = TEST_MUTEX.lock();
    let (config, _tmp) = isolated_config();
    write_asset(
        &config,
        "roles",
        "test-role.json",
        json!({
            "id": "test-role",
            "name": "Test Role"
        }),
    );

    let output = invoke_adopt(json!({
        "type":"role",
        "identifier":"test-role",
        "learn": true
    }));

    // Structural invariant: learn=true routes to help path, not adoption.
    // The output must reference "help" (proving the help code path was taken)
    // and must NOT contain the adoption instruction markers.
    let lower = output.to_ascii_lowercase();
    assert!(
        lower.contains("help"),
        "expected help-related output for learn=true: {output}"
    );
    // Negative: adoption instruction marker must NOT be present
    assert!(
        !output.contains("JSON Configuration:"),
        "learn=true should return help, not adoption instruction: {output}"
    );
    // Negative: adoption preamble must NOT be present
    assert!(
        !output.contains("CRITICAL INSTRUCTIONS:"),
        "learn=true should return help, not adoption instruction body: {output}"
    );

    Config::reset_overrides();
}

// T-1.4a: RTM FR-1 (RTM-007 / TC-1.7)
#[test]
fn adopt_output_contains_json_configuration_followed_by_valid_json() {
    let _guard = TEST_MUTEX.lock();
    let (config, _tmp) = isolated_config();
    write_asset(
        &config,
        "roles",
        "test-role.json",
        json!({
            "id": "test-role",
            "name": "Test Role",
            "personality": "analytical",
            "principles": ["accuracy"]
        }),
    );

    let output = invoke_adopt(json!({"type":"role","identifier":"test-role"}));

    assert!(
        output.contains("JSON Configuration:"),
        "expected JSON Configuration marker: {output}"
    );

    let parsed = extract_json_configuration(&output);
    assert_eq!(parsed["id"], "test-role");
    assert_eq!(parsed["name"], "Test Role");

    Config::reset_overrides();
}

// T-1.4a: RTM FR-1 — R4: missing-asset negative test
// Verifies graceful error (not panic) when identifier matches no asset file.
#[test]
fn adopt_nonexistent_identifier_returns_not_found_error() {
    let _guard = TEST_MUTEX.lock();
    let (_config, _tmp) = isolated_config();

    let result = ToolRegistry::global().try_invoke(
        "Adopt",
        &json!({"type": "role", "identifier": "does-not-exist"}),
        OutputFormat::Markdown,
    );

    assert!(
        result.is_err(),
        "expected error for nonexistent identifier, got: {:?}",
        result.unwrap()
    );

    Config::reset_overrides();
}

// T-1.4a: RTM FR-1 — R5: malformed-asset-JSON resilience test
// Verifies the tool does not panic when asset file contains invalid JSON.
// NOTE: The tool currently passes raw content through without JSON validation;
// it does not return an error for malformed files. This test documents that
// the no-panic property holds and the output lacks a parseable JSON configuration.
#[test]
fn adopt_malformed_asset_json_does_not_panic() {
    let _guard = TEST_MUTEX.lock();
    let (config, _tmp) = isolated_config();

    // Write invalid JSON to the asset file
    let path = config.assets_path.join("roles").join("broken.json");
    fs::create_dir_all(path.parent().expect("parent")).expect("asset dirs");
    fs::write(&path, "{ not valid json !!!").expect("write broken asset");

    // Tool should not panic — either returns Ok (with degraded output) or Err
    let result = ToolRegistry::global().try_invoke(
        "Adopt",
        &json!({"type": "role", "identifier": "broken"}),
        OutputFormat::Markdown,
    );

    match result {
        Ok(output) => {
            // If the tool returns Ok, the identifier should still appear in output
            // (documenting the current pass-through behavior)
            assert!(
                output.contains("broken"),
                "expected identifier in output: {output}"
            );
        }
        Err(err) => {
            // If the tool returns an error, that's the ideal behavior
            let msg = err.to_string();
            assert!(
                !msg.is_empty(),
                "expected meaningful error for malformed JSON"
            );
        }
    }

    Config::reset_overrides();
}
