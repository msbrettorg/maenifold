// T-12.7: RTM NFR-1
#[path = "session/support.rs"]
mod support;

use support::*;

use serde_json::json;

#[test]
fn update_assumption_rejects_invalid_status() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "append",
            "assumption": "Test assumption",
            "concepts": ["test"]
        }),
    );
    let uri = extract_memory_uri(&create_result);

    let result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "update",
            "uri": uri,
            "status": "invalid-status"
        }),
    );

    assert!(result.starts_with("ERROR: Invalid status"));
    assert!(result.contains("active, validated, invalidated, refined"));
}

#[test]
fn update_assumption_adds_timestamped_notes() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "append",
            "assumption": "Test assumption for notes",
            "concepts": ["test-notes"]
        }),
    );
    let uri = extract_memory_uri(&create_result);

    let update_result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "update",
            "uri": uri,
            "notes": "Additional context added"
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
    assert!(read_result.contains("## Update:"));
    assert!(read_result.contains("Additional context added"));
}

#[test]
fn read_non_existent_assumption_returns_error() {
    let harness = TestHarness::new();
    let result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "read",
            "uri": "memory://assumptions/2025/01/assumption-nonexistent"
        }),
    );

    assert!(result.starts_with("ERROR: Assumption not found"));
}

#[test]
fn update_non_existent_assumption_returns_error() {
    let harness = TestHarness::new();
    let result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "update",
            "uri": "memory://assumptions/2025/01/assumption-nonexistent",
            "status": "validated"
        }),
    );

    assert!(result.starts_with("ERROR: Assumption not found"));
}

#[test]
fn invalid_action_returns_error() {
    let harness = TestHarness::new();
    let result = harness.invoke_markdown("AssumptionLedger", json!({ "action": "invalid-action" }));

    assert!(result.starts_with("ERROR: Invalid action"));
    assert!(result.contains("'append', 'update', or 'read'"));
}

#[test]
fn assumption_file_uses_correct_date_based_path() {
    let harness = TestHarness::new();
    let now = Utc::now();
    let year = format!("{:04}", now.year());
    let month = format!("{:02}", now.month());
    let date_dir = harness
        .config
        .memory_path
        .join("assumptions")
        .join(&year)
        .join(&month);
    let files_before = std::fs::read_dir(&date_dir)
        .ok()
        .map(|entries| entries.count())
        .unwrap_or(0);

    let result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "append",
            "assumption": "Test date path",
            "concepts": ["test"]
        }),
    );

    assert!(result.contains(&format!("memory://assumptions/{year}/{month}/assumption-")));
    let files_after = std::fs::read_dir(&date_dir).expect("date dir").count();
    assert_eq!(files_after, files_before + 1);
}

#[test]
fn update_assumption_changes_confidence() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "append",
            "assumption": "Test confidence update",
            "concepts": ["test"],
            "confidence": "low"
        }),
    );
    let uri = extract_memory_uri(&create_result);

    let update_result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "update",
            "uri": uri,
            "confidence": "high"
        }),
    );

    assert!(update_result.contains("**New Confidence**: high"));
    let read_result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "read",
            "uri": uri
        }),
    );
    assert!(read_result.contains("**Confidence**: high"));
}
