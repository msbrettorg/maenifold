use maenifold_core::json::{SAFE_JSON_MAX_DEPTH, from_str_safe};
use serde_json::Value;

fn create_nested_json(depth: usize) -> String {
    let mut json = "\"value\"".to_string();
    for _ in 0..depth {
        json = format!("{{\"nested\":{json}}}");
    }
    json
}

// T-0.1: RTM NFR-3 / SEC-001 verifies safe JSON parsing accepts well-bounded nesting.
#[test]
fn safe_json_with_normal_depth_deserializes() {
    let json = create_nested_json(10);

    let parsed = from_str_safe::<Value>(&json);

    assert!(parsed.is_ok(), "expected successful parse, got: {parsed:?}");
}

// T-0.1: RTM NFR-3 / SEC-001 preserves the C# SafeJson max-depth boundary at 32.
#[test]
fn safe_json_with_max_depth_32_deserializes() {
    let json = create_nested_json(SAFE_JSON_MAX_DEPTH);

    let parsed = from_str_safe::<Value>(&json);

    assert!(parsed.is_ok(), "expected successful parse, got: {parsed:?}");
}

// T-0.1: RTM NFR-3 / SEC-001 rejects payloads deeper than the configured nesting limit.
#[test]
fn safe_json_with_depth_over_32_throws_exception() {
    let json = create_nested_json(SAFE_JSON_MAX_DEPTH + 1);

    let error = from_str_safe::<Value>(&json).expect_err("depth 33 should fail");

    assert!(
        error.to_string().to_ascii_lowercase().contains("depth"),
        "expected depth-related error, got: {error}"
    );
}

// T-0.1: RTM NFR-3 / SEC-001 rejects extreme nesting to guard against parser DoS.
#[test]
fn safe_json_with_depth_64_throws_exception() {
    let json = create_nested_json(64);

    let error = from_str_safe::<Value>(&json).expect_err("depth 64 should fail");

    assert!(
        error.to_string().to_ascii_lowercase().contains("depth"),
        "expected depth-related error, got: {error}"
    );
}
