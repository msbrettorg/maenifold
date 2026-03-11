// T-12.7: RTM NFR-1
#[path = "session/support.rs"]
mod support;

use support::*;

use serde_json::json;

#[test]
fn append_assumption_creates_file_with_correct_structure() {
    let harness = TestHarness::new();
    let assumption = "The dialogue tool will remain MCP-only";
    let confidence = "medium";
    let validation_plan = "Validate once dialogue MCP hooks are reintroduced";
    let context = "workflow://thinking/session-1756610546730";
    let now = Utc::now();
    let date_dir = harness
        .config
        .memory_path
        .join("assumptions")
        .join(format!("{:04}", now.year()))
        .join(format!("{:02}", now.month()));
    let files_before = std::fs::read_dir(&date_dir)
        .ok()
        .map(|entries| entries.count())
        .unwrap_or(0);

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
    assert!(result.contains("💡 Next: Consider running Sync()"));

    let files_after = std::fs::read_dir(&date_dir).expect("date dir").count();
    assert_eq!(files_after, files_before + 1);

    let uri = extract_memory_uri(&result);
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
fn append_assumption_requires_concepts() {
    let harness = TestHarness::new();
    let result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "append",
            "assumption": "Test assumption",
            "concepts": []
        }),
    );

    assert!(result.starts_with("ERROR: At least one [[WikiLink]] tag must be provided"));
}

#[test]
fn append_assumption_rejects_concepts_with_brackets() {
    let harness = TestHarness::new();
    let result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "append",
            "assumption": "Test assumption",
            "concepts": ["[[dialogue]]"]
        }),
    );

    assert!(result.starts_with("ERROR: Concept tags should not include brackets"));
}

#[test]
fn read_assumption_returns_correct_content() {
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
    assert!(result.contains("**Created**:"));
    assert!(result.contains("**Modified**:"));
    assert!(result.contains("**Validation Plan**: Verify with [[test-concept]] follow-up"));
    assert!(result.contains("## Content"));
    assert!(result.contains("Test assumption for reading"));
    assert!(result.contains("[[test-concept]]"));
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

    assert!(read_result.contains("**Validation Plan**: Confirm via [[test-update]] review"));
}
