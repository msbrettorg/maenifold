// T-COV-001.8: RTM FR-17.10
//
//! MCP resource access tests — ported from C#
//! `Maenifold.Tests/McpResourceToolsTests.cs` (both `McpResourceToolsTests`
//! and the embedded `McpResourceToolsEntryPointTests` classes).
//!
//! Ma Protocol Compliance: These tests use REAL systems only.
//! - Real AssetManager and asset initialization (no mocks)
//! - Real CLI binary invocation via `assert_cmd`
//! - Real JSON parsing and validation
//! - No mocks, no stubs, real resource behavior
//!
//! Behavioral differences from C#:
//! - C# throws exceptions (FileNotFoundException, McpException); Rust CLI
//!   returns JSON error envelopes with specific error codes.
//! - C# `ListAssets("")` / `ListAssets("   ")` treats empty/whitespace as null
//!   and returns the primary type array; Rust treats them as literal type
//!   strings that fail `AssetKind::parse`, returning INVALID_ARGUMENT.

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Test harness helpers
// ---------------------------------------------------------------------------

fn isolated_root() -> TempDir {
    TempDir::new().expect("tmp")
}

fn maenifold_cmd(tmp: &TempDir) -> Command {
    // T-COV-001.8: RTM FR-17.10
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("maenifold"));
    cmd.env("MAENIFOLD_ROOT", tmp.path())
        .env("MAENIFOLD_DATABASE_PATH", tmp.path().join("memory.db"));
    cmd
}

/// Invoke a tool in `--json` mode and return parsed JSON envelope.
fn invoke_json(tmp: &TempDir, tool: &str, payload: &str) -> (Value, i32) {
    let output = maenifold_cmd(tmp)
        .args(["--tool", tool, "--payload", payload, "--json"])
        .output()
        .expect("run maenifold");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let value: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_| panic!("expected JSON output, got: {stdout}"));
    (value, code)
}

/// Invoke a tool in `--json` mode and assert success. Return parsed `data`
/// field (which is itself a JSON string that gets parsed).
fn invoke_ok(tmp: &TempDir, tool: &str, payload: &str) -> Value {
    let (envelope, code) = invoke_json(tmp, tool, payload);
    assert_eq!(
        code, 0,
        "expected exit code 0, got {code}; envelope = {envelope}"
    );
    assert_eq!(
        envelope["success"], true,
        "expected success=true; envelope = {envelope}"
    );
    let data_str = envelope["data"]
        .as_str()
        .unwrap_or_else(|| panic!("data should be a string; envelope = {envelope}"));
    serde_json::from_str(data_str)
        .unwrap_or_else(|_| panic!("data should be valid JSON; raw = {data_str}"))
}

/// Invoke a tool in `--json` mode and assert failure. Return the error object.
fn invoke_err(tmp: &TempDir, tool: &str, payload: &str) -> Value {
    let (envelope, code) = invoke_json(tmp, tool, payload);
    assert_ne!(code, 0, "expected non-zero exit code; envelope = {envelope}");
    assert_eq!(
        envelope["success"], false,
        "expected success=false; envelope = {envelope}"
    );
    envelope["error"].clone()
}

// ===========================================================================
// McpResourceToolsTests — catalog and direct resource retrieval
// (Ported from C# McpResourceToolsTests class)
// ===========================================================================

// T-COV-001.8: RTM FR-17.10
/// Verify that the asset catalog (via ReadMcpResource "asset://catalog")
/// returns valid JSON containing all expected top-level categories.
#[test]
fn get_catalog_returns_valid_json_with_all_categories() {
    let tmp = isolated_root();
    let catalog = invoke_ok(&tmp, "ReadMcpResource", r#"{"uri":"asset://catalog"}"#);

    // T-WF-SCHEMA-001.1: RTM FR-27.x — workflows split by multiAgent
    assert!(
        catalog.get("singleAgentWorkflows").is_some(),
        "Catalog should contain 'singleAgentWorkflows'"
    );
    assert!(
        catalog.get("multiAgentWorkflows").is_some(),
        "Catalog should contain 'multiAgentWorkflows'"
    );
    assert!(
        catalog.get("roles").is_some(),
        "Catalog should contain 'roles'"
    );
    assert!(
        catalog.get("colors").is_some(),
        "Catalog should contain 'colors'"
    );
    assert!(
        catalog.get("perspectives").is_some(),
        "Catalog should contain 'perspectives'"
    );
}

// T-COV-001.8: RTM FR-17.10
/// Verify that catalog asset lists are non-empty for each category.
#[test]
fn get_catalog_contains_actual_assets() {
    let tmp = isolated_root();
    let catalog = invoke_ok(&tmp, "ReadMcpResource", r#"{"uri":"asset://catalog"}"#);

    // T-WF-SCHEMA-001.1: RTM FR-27.x
    let sa = catalog["singleAgentWorkflows"]
        .as_array()
        .expect("singleAgentWorkflows array");
    assert!(
        !sa.is_empty(),
        "Should have at least one single-agent workflow"
    );

    let ma = catalog["multiAgentWorkflows"]
        .as_array()
        .expect("multiAgentWorkflows array");
    assert!(
        !ma.is_empty(),
        "Should have at least one multi-agent workflow"
    );

    let roles = catalog["roles"].as_array().expect("roles array");
    assert!(!roles.is_empty(), "Should have at least one role");

    let colors = catalog["colors"].as_array().expect("colors array");
    assert!(!colors.is_empty(), "Should have at least one color");

    let perspectives = catalog["perspectives"]
        .as_array()
        .expect("perspectives array");
    assert!(
        !perspectives.is_empty(),
        "Should have at least one perspective"
    );
}

// T-COV-001.8: RTM FR-17.10
/// Verify that catalog asset entries have required metadata fields (id, name).
#[test]
fn get_catalog_asset_metadata_is_complete() {
    let tmp = isolated_root();
    let catalog = invoke_ok(&tmp, "ReadMcpResource", r#"{"uri":"asset://catalog"}"#);

    // T-WF-SCHEMA-001.1: RTM FR-27.x
    let workflows = catalog["singleAgentWorkflows"]
        .as_array()
        .expect("singleAgentWorkflows array");
    let first = &workflows[0];

    let id = first["id"].as_str().expect("workflow should have 'id'");
    assert!(!id.is_empty(), "Asset id should not be empty");

    let name = first["name"]
        .as_str()
        .expect("workflow should have 'name'");
    assert!(!name.is_empty(), "Asset name should not be empty");

    // emoji and description are optional but if present should be non-empty
    if let Some(emoji) = first.get("emoji").and_then(Value::as_str) {
        assert!(!emoji.is_empty(), "If asset has emoji, it should not be empty");
    }
    if let Some(desc) = first.get("description").and_then(Value::as_str) {
        assert!(
            !desc.is_empty(),
            "If asset has description, it should not be empty"
        );
    }
}

// T-COV-001.8: RTM FR-17.10
/// Verify that retrieving a workflow by URI returns valid JSON with matching id.
#[test]
fn get_workflow_with_valid_id_returns_json_with_id_and_name_fields() {
    let tmp = isolated_root();
    let catalog = invoke_ok(&tmp, "ReadMcpResource", r#"{"uri":"asset://catalog"}"#);

    let workflows = catalog["singleAgentWorkflows"]
        .as_array()
        .expect("singleAgentWorkflows");
    let first_id = workflows[0]["id"].as_str().expect("workflow id");

    let uri = format!("asset://workflows/{first_id}");
    let payload = serde_json::json!({"uri": uri}).to_string();
    let workflow = invoke_ok(&tmp, "ReadMcpResource", &payload);

    assert_eq!(
        workflow["id"].as_str(),
        Some(first_id),
        "Workflow id should match requested ID"
    );
    let name = workflow["name"]
        .as_str()
        .expect("Workflow should have 'name' field");
    assert!(!name.is_empty(), "Workflow name should not be empty");
}

// T-COV-001.8: RTM FR-17.10
/// Verify that retrieving a role by URI returns valid JSON with matching id.
#[test]
fn get_role_with_valid_id_returns_valid_json() {
    let tmp = isolated_root();
    let catalog = invoke_ok(&tmp, "ReadMcpResource", r#"{"uri":"asset://catalog"}"#);

    let roles = catalog["roles"].as_array().expect("roles");
    let first_id = roles[0]["id"].as_str().expect("role id");

    let uri = format!("asset://roles/{first_id}");
    let payload = serde_json::json!({"uri": uri}).to_string();
    let role = invoke_ok(&tmp, "ReadMcpResource", &payload);

    assert_eq!(
        role["id"].as_str(),
        Some(first_id),
        "Role id should match requested ID"
    );
}

// T-COV-001.8: RTM FR-17.10
/// Verify that retrieving a color by URI returns valid JSON with matching id.
#[test]
fn get_color_with_valid_id_returns_valid_json() {
    let tmp = isolated_root();
    let catalog = invoke_ok(&tmp, "ReadMcpResource", r#"{"uri":"asset://catalog"}"#);

    let colors = catalog["colors"].as_array().expect("colors");
    let first_id = colors[0]["id"].as_str().expect("color id");

    let uri = format!("asset://colors/{first_id}");
    let payload = serde_json::json!({"uri": uri}).to_string();
    let color = invoke_ok(&tmp, "ReadMcpResource", &payload);

    assert_eq!(
        color["id"].as_str(),
        Some(first_id),
        "Color id should match requested ID"
    );
}

// T-COV-001.8: RTM FR-17.10
/// Verify that retrieving a perspective by URI returns valid JSON with matching id.
#[test]
fn get_perspective_with_valid_id_returns_valid_json() {
    let tmp = isolated_root();
    let catalog = invoke_ok(&tmp, "ReadMcpResource", r#"{"uri":"asset://catalog"}"#);

    let perspectives = catalog["perspectives"].as_array().expect("perspectives");
    let first_id = perspectives[0]["id"].as_str().expect("perspective id");

    let uri = format!("asset://perspectives/{first_id}");
    let payload = serde_json::json!({"uri": uri}).to_string();
    let perspective = invoke_ok(&tmp, "ReadMcpResource", &payload);

    assert_eq!(
        perspective["id"].as_str(),
        Some(first_id),
        "Perspective id should match requested ID"
    );
}

// T-COV-001.8: RTM FR-17.10
/// Verify that requesting a non-existent workflow ID returns an error.
/// (C# throws FileNotFoundException; Rust returns INTERNAL_ERROR with
/// "Asset not found" message.)
#[test]
fn get_workflow_with_nonexistent_id_returns_error() {
    let tmp = isolated_root();
    let payload = r#"{"uri":"asset://workflows/non-existent-workflow-xyz"}"#;
    let err = invoke_err(&tmp, "ReadMcpResource", payload);

    let msg = err["message"].as_str().expect("error message");
    assert!(
        msg.contains("Asset not found"),
        "Error should mention 'Asset not found', got: {msg}"
    );
    assert!(
        msg.contains("non-existent-workflow-xyz"),
        "Error should contain the requested ID, got: {msg}"
    );
}

// T-COV-001.8: RTM FR-17.10
/// Verify that requesting a non-existent role ID returns an error.
#[test]
fn get_role_with_nonexistent_id_returns_error() {
    let tmp = isolated_root();
    let payload = r#"{"uri":"asset://roles/non-existent-role-xyz"}"#;
    let err = invoke_err(&tmp, "ReadMcpResource", payload);

    let msg = err["message"].as_str().expect("error message");
    assert!(
        msg.contains("Asset not found"),
        "Error should mention 'Asset not found', got: {msg}"
    );
}

// T-COV-001.8: RTM FR-17.10
/// Verify that requesting a non-existent color ID returns an error.
#[test]
fn get_color_with_nonexistent_id_returns_error() {
    let tmp = isolated_root();
    let payload = r#"{"uri":"asset://colors/non-existent-color-xyz"}"#;
    let err = invoke_err(&tmp, "ReadMcpResource", payload);

    let msg = err["message"].as_str().expect("error message");
    assert!(
        msg.contains("Asset not found"),
        "Error should mention 'Asset not found', got: {msg}"
    );
}

// T-COV-001.8: RTM FR-17.10
/// Verify that requesting a non-existent perspective ID returns an error.
#[test]
fn get_perspective_with_nonexistent_id_returns_error() {
    let tmp = isolated_root();
    let payload = r#"{"uri":"asset://perspectives/non-existent-perspective-xyz"}"#;
    let err = invoke_err(&tmp, "ReadMcpResource", payload);

    let msg = err["message"].as_str().expect("error message");
    assert!(
        msg.contains("Asset not found"),
        "Error should mention 'Asset not found', got: {msg}"
    );
}

// T-COV-001.8: RTM FR-17.10
/// Verify that catalog JSON is formatted (indented with newlines and spaces).
#[test]
fn get_catalog_returns_indented_json() {
    let tmp = isolated_root();
    let (envelope, _) = invoke_json(&tmp, "ReadMcpResource", r#"{"uri":"asset://catalog"}"#);
    let data_str = envelope["data"].as_str().expect("data string");

    // The rendered catalog JSON string should be indented (contain newlines and
    // leading whitespace) — same assertion as the C# test.
    assert!(
        data_str.contains('\n'),
        "JSON should be indented with newlines"
    );
    assert!(
        data_str.contains("  "),
        "JSON should be indented with spaces"
    );
}

// T-COV-001.8: RTM FR-17.10
/// End-to-end consistency: discover all assets via ListAssets, then retrieve
/// each one individually via ReadMcpResource and verify the id round-trips.
///
/// NOTE: Uses ListAssets for discovery (not the catalog) because the catalog
/// may source from a sibling C# project root via `preferred_catalog_assets_root`,
/// while ReadMcpResource reads from the config assets path. ListAssets shares
/// the same root as ReadMcpResource, ensuring the discovered IDs are
/// retrievable.
///
/// (Ported from C# AssetDiscoveryAndRetrievalFlow_IsConsistent)
#[test]
fn asset_discovery_and_retrieval_flow_is_consistent() {
    let tmp = isolated_root();

    // Workflows
    let workflows = invoke_ok(&tmp, "ListAssets", r#"{"type":"workflow"}"#);
    let wf_arr = workflows.as_array().expect("workflows array");
    assert!(!wf_arr.is_empty(), "Should have at least one workflow");

    for wf in wf_arr {
        let id = wf["id"].as_str().expect("workflow id");
        let uri = format!("asset://workflows/{id}");
        let payload = serde_json::json!({"uri": uri}).to_string();
        let fetched = invoke_ok(&tmp, "ReadMcpResource", &payload);
        assert!(
            fetched.get("id").is_some(),
            "Workflow '{id}' should have 'id' field when retrieved"
        );
    }

    // Roles
    let roles = invoke_ok(&tmp, "ListAssets", r#"{"type":"role"}"#);
    let role_arr = roles.as_array().expect("roles array");
    for role in role_arr {
        let id = role["id"].as_str().expect("role id");
        let uri = format!("asset://roles/{id}");
        let payload = serde_json::json!({"uri": uri}).to_string();
        let fetched = invoke_ok(&tmp, "ReadMcpResource", &payload);
        assert!(
            fetched.get("id").is_some(),
            "Role '{id}' should have 'id' field"
        );
    }
}

// ===========================================================================
// McpResourceToolsEntryPointTests — ListAssets + ReadMcpResource entry points
// (Ported from C# McpResourceToolsEntryPointTests class)
// ===========================================================================

// ---------------------------------------------------------------------------
// ListAssets — null / empty / whitespace input
// ---------------------------------------------------------------------------

// T-COV-001.8: RTM FR-17.10
/// ListAssets with no type field returns the JSON array of primary asset type
/// strings: ["workflow", "role", "color", "perspective"].
#[test]
fn list_assets_null_type_returns_primary_type_array() {
    let tmp = isolated_root();
    let result = invoke_ok(&tmp, "ListAssets", "{}");

    let arr = result.as_array().expect("result should be a JSON array");
    let types: Vec<&str> = arr.iter().filter_map(Value::as_str).collect();
    assert!(types.contains(&"workflow"), "Should contain 'workflow'");
    assert!(types.contains(&"role"), "Should contain 'role'");
    assert!(types.contains(&"color"), "Should contain 'color'");
    assert!(
        types.contains(&"perspective"),
        "Should contain 'perspective'"
    );
}

// T-COV-001.8: RTM FR-17.10
/// ListAssets with empty string type.
/// NOTE: C# treats empty string identically to null (returns type array).
/// Rust treats it as a literal type string and fails AssetKind::parse.
#[test]
fn list_assets_empty_string_returns_invalid_argument() {
    let tmp = isolated_root();
    let err = invoke_err(&tmp, "ListAssets", r#"{"type":""}"#);
    assert_eq!(
        err["code"].as_str(),
        Some("INVALID_ARGUMENT"),
        "Empty type should return INVALID_ARGUMENT"
    );
    assert!(
        err["message"]
            .as_str()
            .unwrap_or("")
            .contains("Unknown asset type"),
        "Error should mention 'Unknown asset type'"
    );
}

// T-COV-001.8: RTM FR-17.10
/// ListAssets with whitespace-only string type.
/// NOTE: C# treats whitespace identically to null (returns type array).
/// Rust treats it as a literal type string and fails AssetKind::parse.
#[test]
fn list_assets_whitespace_string_returns_invalid_argument() {
    let tmp = isolated_root();
    let err = invoke_err(&tmp, "ListAssets", r#"{"type":"   "}"#);
    assert_eq!(
        err["code"].as_str(),
        Some("INVALID_ARGUMENT"),
        "Whitespace type should return INVALID_ARGUMENT"
    );
}

// ---------------------------------------------------------------------------
// ListAssets — valid type variants
// ---------------------------------------------------------------------------

// T-COV-001.8: RTM FR-17.10
/// ListAssets("workflow") returns a JSON array of workflow metadata objects.
#[test]
fn list_assets_workflow_singular_returns_workflow_metadata_array() {
    let tmp = isolated_root();
    let result = invoke_ok(&tmp, "ListAssets", r#"{"type":"workflow"}"#);

    let arr = result.as_array().expect("result should be a JSON array");
    assert!(!arr.is_empty(), "Should list at least one workflow");

    // Verify metadata shape
    let first = &arr[0];
    assert!(
        first.get("id").is_some(),
        "Workflow metadata should have 'id'"
    );
    assert!(
        first.get("name").is_some(),
        "Workflow metadata should have 'name'"
    );
}

// T-COV-001.8: RTM FR-17.10
/// ListAssets("workflows") (plural) returns the same result as singular
/// "workflow".
#[test]
fn list_assets_workflow_plural_returns_same_as_singular() {
    let tmp = isolated_root();
    let singular = invoke_ok(&tmp, "ListAssets", r#"{"type":"workflow"}"#);
    let plural = invoke_ok(&tmp, "ListAssets", r#"{"type":"workflows"}"#);

    assert_eq!(
        singular, plural,
        "Plural 'workflows' should return same result as singular 'workflow'"
    );
}

// T-COV-001.8: RTM FR-17.10
/// ListAssets("role") returns a JSON array of role metadata objects.
#[test]
fn list_assets_role_returns_role_metadata_array() {
    let tmp = isolated_root();
    let result = invoke_ok(&tmp, "ListAssets", r#"{"type":"role"}"#);

    let arr = result.as_array().expect("result should be a JSON array");
    assert!(!arr.is_empty(), "Should list at least one role");
}

// T-COV-001.8: RTM FR-17.10
/// ListAssets("color") returns a JSON array of color metadata objects.
#[test]
fn list_assets_color_returns_color_metadata_array() {
    let tmp = isolated_root();
    let result = invoke_ok(&tmp, "ListAssets", r#"{"type":"color"}"#);

    let arr = result.as_array().expect("result should be a JSON array");
    assert!(!arr.is_empty(), "Should list at least one color");
}

// T-COV-001.8: RTM FR-17.10
/// ListAssets("perspective") returns a JSON array of perspective metadata
/// objects.
#[test]
fn list_assets_perspective_returns_perspective_metadata_array() {
    let tmp = isolated_root();
    let result = invoke_ok(&tmp, "ListAssets", r#"{"type":"perspective"}"#);

    let arr = result.as_array().expect("result should be a JSON array");
    assert!(!arr.is_empty(), "Should list at least one perspective");
}

// T-COV-001.8: RTM FR-17.10
/// ListAssets is case-insensitive: "WORKFLOW" resolves identically to
/// "workflow".
#[test]
fn list_assets_uppercase_type_is_case_insensitive() {
    let tmp = isolated_root();
    let lower = invoke_ok(&tmp, "ListAssets", r#"{"type":"workflow"}"#);
    let upper = invoke_ok(&tmp, "ListAssets", r#"{"type":"WORKFLOW"}"#);

    assert_eq!(
        lower, upper,
        "'WORKFLOW' should resolve identically to 'workflow'"
    );
}

// ---------------------------------------------------------------------------
// ListAssets — invalid type
// ---------------------------------------------------------------------------

// T-COV-001.8: RTM FR-17.10
/// ListAssets with an unknown type returns INVALID_ARGUMENT error containing
/// "Unknown asset type".
/// (C# throws McpException; Rust returns JSON error envelope.)
#[test]
fn list_assets_unknown_type_returns_error() {
    let tmp = isolated_root();
    let err = invoke_err(&tmp, "ListAssets", r#"{"type":"bogus"}"#);

    assert_eq!(err["code"].as_str(), Some("INVALID_ARGUMENT"));
    let msg = err["message"].as_str().expect("error message");
    assert!(
        msg.contains("Unknown asset type"),
        "Error should contain 'Unknown asset type', got: {msg}"
    );
    assert!(
        msg.contains("bogus"),
        "Error should reference the supplied type 'bogus', got: {msg}"
    );
}

// ---------------------------------------------------------------------------
// ReadMcpResource — null / empty URI
// ---------------------------------------------------------------------------

// T-COV-001.8: RTM FR-17.10
/// ReadMcpResource with no URI field returns INVALID_ARGUMENT error.
/// (C# throws McpException "URI is required"; Rust returns "Required property
/// 'uri' is missing from payload.")
#[test]
fn read_mcp_resource_missing_uri_returns_error() {
    let tmp = isolated_root();
    let err = invoke_err(&tmp, "ReadMcpResource", "{}");

    assert_eq!(err["code"].as_str(), Some("INVALID_ARGUMENT"));
    let msg = err["message"].as_str().expect("error message");
    assert!(
        msg.contains("uri"),
        "Error should reference 'uri' property, got: {msg}"
    );
}

// T-COV-001.8: RTM FR-17.10
/// ReadMcpResource with empty string URI returns error about invalid URI
/// format.
/// (C# throws McpException "URI is required"; Rust parses the empty string and
/// returns "Invalid resource URI format".)
#[test]
fn read_mcp_resource_empty_uri_returns_error() {
    let tmp = isolated_root();
    let err = invoke_err(&tmp, "ReadMcpResource", r#"{"uri":""}"#);

    let msg = err["message"].as_str().expect("error message");
    assert!(
        msg.contains("Invalid resource URI format"),
        "Error should contain 'Invalid resource URI format', got: {msg}"
    );
}

// ---------------------------------------------------------------------------
// ReadMcpResource — catalog URI
// ---------------------------------------------------------------------------

// T-COV-001.8: RTM FR-17.10
/// ReadMcpResource("asset://catalog") returns valid catalog JSON with all asset
/// categories.
#[test]
fn read_mcp_resource_catalog_uri_returns_catalog_json() {
    let tmp = isolated_root();
    let result = invoke_ok(&tmp, "ReadMcpResource", r#"{"uri":"asset://catalog"}"#);

    // T-WF-SCHEMA-001.1: RTM FR-27.x — workflows split by multiAgent
    assert!(
        result.get("singleAgentWorkflows").is_some(),
        "Catalog should contain 'singleAgentWorkflows'"
    );
    assert!(
        result.get("multiAgentWorkflows").is_some(),
        "Catalog should contain 'multiAgentWorkflows'"
    );
    assert!(result.get("roles").is_some(), "Catalog should contain 'roles'");
    assert!(
        result.get("colors").is_some(),
        "Catalog should contain 'colors'"
    );
    assert!(
        result.get("perspectives").is_some(),
        "Catalog should contain 'perspectives'"
    );
}

// ---------------------------------------------------------------------------
// ReadMcpResource — valid asset URIs
// ---------------------------------------------------------------------------

// T-COV-001.8: RTM FR-17.10
/// ReadMcpResource with a valid workflow URI returns the workflow JSON with
/// matching id.
#[test]
fn read_mcp_resource_valid_workflow_uri_returns_workflow_json() {
    let tmp = isolated_root();
    let catalog = invoke_ok(&tmp, "ReadMcpResource", r#"{"uri":"asset://catalog"}"#);

    let workflows = catalog["singleAgentWorkflows"]
        .as_array()
        .expect("singleAgentWorkflows");
    assert!(
        !workflows.is_empty(),
        "Need at least one workflow in catalog"
    );

    let wf_id = workflows[0]["id"].as_str().expect("workflow id");
    let uri = format!("asset://workflows/{wf_id}");
    let payload = serde_json::json!({"uri": uri}).to_string();
    let result = invoke_ok(&tmp, "ReadMcpResource", &payload);

    assert_eq!(
        result["id"].as_str(),
        Some(wf_id),
        "Returned workflow id should match requested id"
    );
}

// T-COV-001.8: RTM FR-17.10
/// ReadMcpResource with a valid role URI returns the role JSON.
#[test]
fn read_mcp_resource_valid_role_uri_returns_role_json() {
    let tmp = isolated_root();
    let catalog = invoke_ok(&tmp, "ReadMcpResource", r#"{"uri":"asset://catalog"}"#);

    let roles = catalog["roles"].as_array().expect("roles");
    assert!(!roles.is_empty(), "Need at least one role in catalog");

    let role_id = roles[0]["id"].as_str().expect("role id");
    let uri = format!("asset://roles/{role_id}");
    let payload = serde_json::json!({"uri": uri}).to_string();
    let result = invoke_ok(&tmp, "ReadMcpResource", &payload);

    assert!(
        result.get("id").is_some(),
        "Role JSON should have 'id' field"
    );
}

// ---------------------------------------------------------------------------
// ReadMcpResource — invalid URI format
// ---------------------------------------------------------------------------

// T-COV-001.8: RTM FR-17.10
/// ReadMcpResource with an invalid URI (no asset:// scheme) returns an error
/// with "Invalid resource URI format".
/// (C# throws McpException; Rust returns INTERNAL_ERROR.)
#[test]
fn read_mcp_resource_invalid_uri_format_returns_error() {
    let tmp = isolated_root();
    let err = invoke_err(&tmp, "ReadMcpResource", r#"{"uri":"invalid-uri"}"#);

    let msg = err["message"].as_str().expect("error message");
    assert!(
        msg.contains("Invalid resource URI format"),
        "Error should contain 'Invalid resource URI format', got: {msg}"
    );
}

// T-COV-001.8: RTM FR-17.10
/// ReadMcpResource with "asset://workflows" (missing path segment after type)
/// returns an error.
/// (C# throws McpException "Invalid resource URI format"; Rust also returns
/// "Invalid resource URI format".)
#[test]
fn read_mcp_resource_uri_missing_path_segment_returns_error() {
    let tmp = isolated_root();
    let err = invoke_err(&tmp, "ReadMcpResource", r#"{"uri":"asset://workflows"}"#);

    let msg = err["message"].as_str().expect("error message");
    assert!(
        msg.contains("Invalid resource URI format"),
        "Error should contain 'Invalid resource URI format', got: {msg}"
    );
}

// ---------------------------------------------------------------------------
// ReadMcpResource — unknown resource type
// ---------------------------------------------------------------------------

// T-COV-001.8: RTM FR-17.10
/// ReadMcpResource with an unknown resource type segment returns an error
/// containing "Unknown asset type".
/// (C# says "Unknown resource type"; Rust says "Unknown asset type" via
/// AssetKind::parse.)
#[test]
fn read_mcp_resource_unknown_resource_type_returns_error() {
    let tmp = isolated_root();
    let err = invoke_err(
        &tmp,
        "ReadMcpResource",
        r#"{"uri":"asset://bogus/some-id"}"#,
    );

    let msg = err["message"].as_str().expect("error message");
    assert!(
        msg.contains("Unknown asset type"),
        "Error should contain 'Unknown asset type', got: {msg}"
    );
    assert!(
        msg.contains("bogus"),
        "Error should reference the supplied type 'bogus', got: {msg}"
    );
}

// ---------------------------------------------------------------------------
// ReadMcpResource — nonexistent asset IDs
// ---------------------------------------------------------------------------

// T-COV-001.8: RTM FR-17.10
/// ReadMcpResource with a valid type but nonexistent workflow ID returns error.
/// (C# throws FileNotFoundException; Rust returns "Asset not found".)
#[test]
fn read_mcp_resource_nonexistent_workflow_id_returns_error() {
    let tmp = isolated_root();
    let err = invoke_err(
        &tmp,
        "ReadMcpResource",
        r#"{"uri":"asset://workflows/nonexistent-workflow-xyz-99999"}"#,
    );

    let msg = err["message"].as_str().expect("error message");
    assert!(
        msg.contains("Asset not found"),
        "Error should contain 'Asset not found', got: {msg}"
    );
}

// T-COV-001.8: RTM FR-17.10
/// ReadMcpResource with a valid type but nonexistent role ID returns error.
/// (C# throws FileNotFoundException; Rust returns "Asset not found".)
#[test]
fn read_mcp_resource_nonexistent_role_id_returns_error() {
    let tmp = isolated_root();
    let err = invoke_err(
        &tmp,
        "ReadMcpResource",
        r#"{"uri":"asset://roles/nonexistent-role-xyz-99999"}"#,
    );

    let msg = err["message"].as_str().expect("error message");
    assert!(
        msg.contains("Asset not found"),
        "Error should contain 'Asset not found', got: {msg}"
    );
}
