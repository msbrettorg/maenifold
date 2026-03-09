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

// T-1.4a: RTM FR-1 (RTM-001 / TC-1.1)
#[test]
fn adopt_role_returns_instruction_containing_role_and_identifier() {
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
    let lower = output.to_ascii_lowercase();

    assert!(
        lower.contains("role"),
        "expected role wording in output: {output}"
    );
    assert!(
        output.contains("test-role"),
        "expected identifier in output: {output}"
    );

    Config::reset_overrides();
}

// T-1.4a: RTM FR-1 (RTM-002 / TC-1.2)
#[test]
fn adopt_color_returns_adoption_instruction() {
    let _guard = TEST_MUTEX.lock();
    let (config, _tmp) = isolated_config();
    write_asset(
        &config,
        "colors",
        "test-color.json",
        json!({
            "id": "test-color",
            "name": "Test Color",
            "mode": "critical",
            "principles": ["caution"]
        }),
    );

    let output = invoke_adopt(json!({"type":"color","identifier":"test-color"}));
    let lower = output.to_ascii_lowercase();

    assert!(
        lower.contains("color"),
        "expected color wording in output: {output}"
    );
    assert!(
        output.contains("test-color"),
        "expected identifier in output: {output}"
    );

    Config::reset_overrides();
}

// T-1.4a: RTM FR-1 (RTM-003 / TC-1.3)
#[test]
fn adopt_perspective_returns_adoption_instruction() {
    let _guard = TEST_MUTEX.lock();
    let (config, _tmp) = isolated_config();
    write_asset(
        &config,
        "perspectives",
        "test-perspective.json",
        json!({
            "id": "test-perspective",
            "name": "Test Perspective",
            "voice": "skeptical",
            "principles": ["clarity"]
        }),
    );

    let output = invoke_adopt(json!({
        "type":"perspective",
        "identifier":"test-perspective"
    }));
    let lower = output.to_ascii_lowercase();

    assert!(
        lower.contains("perspective"),
        "expected perspective wording in output: {output}"
    );
    assert!(
        output.contains("test-perspective"),
        "expected identifier in output: {output}"
    );

    Config::reset_overrides();
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

    assert!(
        output.contains("Adopt") || output.contains("Loads") || output.contains("Parameters"),
        "expected help documentation shape: {output}"
    );
    assert!(
        !output.contains("JSON Configuration:"),
        "learn=true should return help, not adoption instruction: {output}"
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
