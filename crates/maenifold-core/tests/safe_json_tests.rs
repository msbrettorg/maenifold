use maenifold_core::json::{SAFE_JSON_MAX_DEPTH, from_str_safe};
use serde_json::Value;

fn create_nested_json(depth: usize) -> String {
    let mut json = "\"value\"".to_string();
    for _ in 0..depth {
        json = format!("{{\"nested\":{json}}}");
    }
    json
}

// T-0.1: RTM NFR-3 — helper: builds a deeply nested JSON array payload.
fn create_nested_array(depth: usize) -> String {
    let mut json = "\"value\"".to_string();
    for _ in 0..depth {
        json = format!("[{json}]");
    }
    json
}

// T-0.1: RTM NFR-3 — parameterized accept-case for depths well below and just under the limit.
#[test]
fn safe_json_sub_boundary_depths_deserialize() {
    for depth in [1, 10, 31] {
        let json = create_nested_json(depth);
        let parsed = from_str_safe::<Value>(&json);
        assert!(
            parsed.is_ok(),
            "depth {depth} should parse successfully, got: {parsed:?}"
        );
    }
}

// T-0.1: RTM NFR-3 — preserves the C# SafeJson max-depth boundary at 32.
#[test]
fn safe_json_with_max_depth_32_deserializes() {
    let json = create_nested_json(SAFE_JSON_MAX_DEPTH);

    let parsed = from_str_safe::<Value>(&json);

    assert!(parsed.is_ok(), "expected successful parse, got: {parsed:?}");
}

// T-0.1: RTM NFR-3 — rejects payloads deeper than the configured nesting limit.
#[test]
fn safe_json_depth_33_exceeds_limit_returns_err() {
    let json = create_nested_json(SAFE_JSON_MAX_DEPTH + 1);

    let error = from_str_safe::<Value>(&json).expect_err("depth 33 should fail");

    assert!(
        error.to_string().to_ascii_lowercase().contains("depth"),
        "expected depth-related error, got: {error}"
    );
}

// T-0.1: RTM NFR-3 — array nesting increments depth the same way object nesting does.
#[test]
fn safe_json_array_nesting_at_max_depth_deserializes() {
    let json = create_nested_array(SAFE_JSON_MAX_DEPTH);

    let parsed = from_str_safe::<Value>(&json);

    assert!(
        parsed.is_ok(),
        "array nesting at depth {SAFE_JSON_MAX_DEPTH} should parse, got: {parsed:?}"
    );
}

// T-0.1: RTM NFR-3 — array nesting beyond the limit is rejected like object nesting.
#[test]
fn safe_json_array_nesting_at_33_returns_err() {
    let json = create_nested_array(SAFE_JSON_MAX_DEPTH + 1);

    let error = from_str_safe::<Value>(&json).expect_err("array depth 33 should fail");

    assert!(
        error.to_string().to_ascii_lowercase().contains("depth"),
        "expected depth-related error for array nesting, got: {error}"
    );
}

// T-0.1: RTM NFR-3 — brackets inside JSON strings must not inflate the depth counter.
#[test]
fn safe_json_brackets_inside_strings_are_not_counted() {
    let json = r#"{"key": "value {with} [brackets] \" escaped"}"#;

    let parsed = from_str_safe::<Value>(json);

    assert!(
        parsed.is_ok(),
        "depth-1 payload with string brackets should parse, got: {parsed:?}"
    );
}

// T-0.1: RTM NFR-3 — mixed object and array nesting at boundary is accepted.
#[test]
fn safe_json_mixed_nesting_at_max_depth_deserializes() {
    // Alternating { and [ to reach exactly SAFE_JSON_MAX_DEPTH.
    let mut json = "\"value\"".to_string();
    for i in 0..SAFE_JSON_MAX_DEPTH {
        if i % 2 == 0 {
            json = format!("[{json}]");
        } else {
            json = format!("{{\"k\":{json}}}");
        }
    }

    let parsed = from_str_safe::<Value>(&json);

    assert!(
        parsed.is_ok(),
        "mixed nesting at depth {SAFE_JSON_MAX_DEPTH} should parse, got: {parsed:?}"
    );
}
