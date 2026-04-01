// T-12.7: RTM NFR-1
#[path = "session/support.rs"]
mod support;

use support::*;

use serde_json::json;

// T-12.7: RTM NFR-1 — shared helper for append input-validation tests.
fn assert_append_validation_error(concepts: Value, expected_prefix: &str) {
    let harness = TestHarness::new();
    let result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "append",
            "assumption": "Test assumption",
            "concepts": concepts
        }),
    );
    assert!(
        result.starts_with(expected_prefix),
        "expected prefix {expected_prefix:?}, got: {result:?}"
    );
}

#[test]
fn append_assumption_creates_file_with_correct_structure() {
    // T-12.7: RTM NFR-1 — REWRITE: removed redundant file-count side-effect assertion;
    // loosened display-copy hint assertion to semantic stability check.
    let harness = TestHarness::new();
    let assumption = "The dialogue tool will remain MCP-only";
    let confidence = "medium";
    let validation_plan = "Validate once dialogue MCP hooks are reintroduced";
    let context = "workflow://thinking/session-1756610546730";

    let result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "append",
            "assumption": assumption,
            "context": context,
            "validationPlan": validation_plan,
            "confidence": confidence,
            "concepts": ["dialogue", "workflow-dispatch"]
        }),
    );

    assert!(result.starts_with("✅ Assumption recorded at memory://assumptions/"));
    assert!(result.contains(&format!("**Statement**: {assumption}")));
    assert!(result.contains(&format!("**Confidence**: {confidence}")));
    assert!(result.contains("**Status**: active"));
    assert!(result.contains(&format!("**Context**: {context}")));
    // Semantic stability: verify no error and URI is a valid assumptions path.
    assert!(!result.contains("ERROR"));
    let uri = extract_memory_uri(&result);
    assert!(uri.starts_with("memory://assumptions/"));

    let (frontmatter, content) = harness.read(&uri);
    assert_eq!(frontmatter_string(&frontmatter, "status"), "active");
    assert_eq!(frontmatter_string(&frontmatter, "confidence"), confidence);
    assert_eq!(frontmatter_string(&frontmatter, "context"), context);
    assert_eq!(
        frontmatter_string(&frontmatter, "validation_plan"),
        validation_plan
    );
    assert!(content.contains("[[dialogue]]"));
    assert!(content.contains("[[workflow-dispatch]]"));
}

#[test]
fn append_assumption_rejects_invalid_concepts() {
    // T-12.7: RTM NFR-1 — MERGE: parameterised from requires_concepts + rejects_concepts_with_brackets.
    assert_append_validation_error(
        json!([]),
        "ERROR: At least one [[WikiLink]] tag must be provided",
    );
    assert_append_validation_error(
        json!(["[[dialogue]]"]),
        "ERROR: Concept tags should not include brackets",
    );
}

#[test]
fn read_assumption_returns_correct_content() {
    // T-12.7: RTM NFR-1 — REWRITE: validate Created/Modified timestamps via frontmatter
    // instead of weak label-presence assertions on markdown output.
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "append",
            "assumption": "Test assumption for reading",
            "validationPlan": "Verify with [[test-concept]] follow-up",
            "concepts": ["test-concept"]
        }),
    );
    let uri = extract_memory_uri(&create_result);

    let result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "read",
            "uri": uri
        }),
    );

    assert!(result.starts_with(&format!("📋 **Assumption**: {uri}")));
    assert!(result.contains("**Status**: active"));
    assert!(result.contains("**Validation Plan**: Verify with [[test-concept]] follow-up"));
    assert!(result.contains("## Content"));
    assert!(result.contains("Test assumption for reading"));
    assert!(result.contains("[[test-concept]]"));

    // Validate Created/Modified timestamps via frontmatter instead of markdown labels.
    let (frontmatter, _) = harness.read(&uri);
    let created = frontmatter_string(&frontmatter, "created");
    let modified = frontmatter_string(&frontmatter, "modified");
    assert!(
        !created.is_empty(),
        "created timestamp must be present in frontmatter"
    );
    assert!(
        !modified.is_empty(),
        "modified timestamp must be present in frontmatter"
    );
    // Validate RFC 3339 / ISO 8601 format.
    let created_dt = chrono::DateTime::parse_from_rfc3339(&created)
        .unwrap_or_else(|_| panic!("created timestamp must be valid RFC 3339: {created}"));
    let modified_dt = chrono::DateTime::parse_from_rfc3339(&modified)
        .unwrap_or_else(|_| panic!("modified timestamp must be valid RFC 3339: {modified}"));
    // Fresh assumption: created and modified must refer to the same instant
    // (no update has occurred). String comparison may differ due to YAML roundtrip
    // normalisation, so compare parsed values within a 1-second tolerance.
    let delta = (created_dt - modified_dt).abs();
    assert!(
        delta < chrono::Duration::seconds(1),
        "freshly-appended assumption must have created ≈ modified, delta: {delta}"
    );
}

#[test]
fn update_assumption_changes_status() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "append",
            "assumption": "Test assumption for updating",
            "concepts": ["test-update"]
        }),
    );
    let uri = extract_memory_uri(&create_result);

    let update_result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "update",
            "uri": uri,
            "status": "validated",
            "notes": "Test validation complete"
        }),
    );

    assert!(update_result.starts_with("✅ Assumption updated at"));
    assert!(update_result.contains("**New Status**: validated"));

    let read_result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "read",
            "uri": uri
        }),
    );

    assert!(read_result.contains("**Status**: validated"));
    assert!(read_result.contains("Test validation complete"));
}

#[test]
fn update_assumption_persists_validation_plan() {
    // T-12.7: RTM NFR-1 — REWRITE: added status-preservation assertion to detect
    // partial updates that silently reset status to a default.
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "append",
            "assumption": "Test assumption for validation plan updates",
            "concepts": ["test-update"]
        }),
    );
    let uri = extract_memory_uri(&create_result);

    let update_result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "update",
            "uri": uri,
            "validationPlan": "Confirm via [[test-update]] review"
        }),
    );

    assert!(update_result.starts_with("✅ Assumption updated at"));

    let read_result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "read",
            "uri": uri
        }),
    );

    // Verify status was not accidentally reset by the partial (validation-plan-only) update.
    assert!(
        read_result.contains("**Status**: active"),
        "partial update must preserve the original status"
    );
    assert!(read_result.contains("**Validation Plan**: Confirm via [[test-update]] review"));
}
