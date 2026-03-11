// T-10.2: RTM FR-11
//! `ListAssets` entry-point tests split from `mcp_resource_tests.rs`.

mod mcp_resource_support;

use mcp_resource_support::{invoke_err, invoke_ok, isolated_root};
use serde_json::Value;

#[test]
fn list_assets_null_type_returns_primary_type_array() {
    let tmp = isolated_root();
    let result = invoke_ok(&tmp, "ListAssets", "{}");

    let arr = result.as_array().expect("result should be a JSON array");
    let types: Vec<&str> = arr.iter().filter_map(Value::as_str).collect();
    assert!(types.contains(&"workflow"));
    assert!(types.contains(&"role"));
    assert!(types.contains(&"color"));
    assert!(types.contains(&"perspective"));
}

#[test]
fn list_assets_empty_string_returns_invalid_argument() {
    let tmp = isolated_root();
    let err = invoke_err(&tmp, "ListAssets", r#"{"type":""}"#);

    assert_eq!(err["code"].as_str(), Some("INVALID_ARGUMENT"));
    assert!(
        err["message"]
            .as_str()
            .unwrap_or("")
            .contains("Unknown asset type")
    );
}

#[test]
fn list_assets_whitespace_string_returns_invalid_argument() {
    let tmp = isolated_root();
    let err = invoke_err(&tmp, "ListAssets", r#"{"type":"   "}"#);

    assert_eq!(err["code"].as_str(), Some("INVALID_ARGUMENT"));
}

#[test]
fn list_assets_workflow_singular_returns_workflow_metadata_array() {
    let tmp = isolated_root();
    let result = invoke_ok(&tmp, "ListAssets", r#"{"type":"workflow"}"#);

    let arr = result.as_array().expect("result should be a JSON array");
    assert!(!arr.is_empty());
    let first = &arr[0];
    assert!(first.get("id").is_some());
    assert!(first.get("name").is_some());
}

#[test]
fn list_assets_workflow_plural_returns_same_as_singular() {
    let tmp = isolated_root();
    let singular = invoke_ok(&tmp, "ListAssets", r#"{"type":"workflow"}"#);
    let plural = invoke_ok(&tmp, "ListAssets", r#"{"type":"workflows"}"#);

    assert_eq!(singular, plural);
}

#[test]
fn list_assets_role_returns_role_metadata_array() {
    let tmp = isolated_root();
    let result = invoke_ok(&tmp, "ListAssets", r#"{"type":"role"}"#);

    let arr = result.as_array().expect("result should be a JSON array");
    assert!(!arr.is_empty());
}

#[test]
fn list_assets_uppercase_type_is_case_insensitive() {
    let tmp = isolated_root();
    let lower = invoke_ok(&tmp, "ListAssets", r#"{"type":"workflow"}"#);
    let upper = invoke_ok(&tmp, "ListAssets", r#"{"type":"WORKFLOW"}"#);

    assert_eq!(lower, upper);
}

#[test]
fn list_assets_unknown_type_returns_error() {
    let tmp = isolated_root();
    let err = invoke_err(&tmp, "ListAssets", r#"{"type":"bogus"}"#);

    assert_eq!(err["code"].as_str(), Some("INVALID_ARGUMENT"));
    let msg = err["message"].as_str().expect("error message");
    assert!(msg.contains("Unknown asset type"));
    assert!(msg.contains("bogus"));
}
