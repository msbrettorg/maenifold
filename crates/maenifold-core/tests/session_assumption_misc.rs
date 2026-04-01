// T-12.7: RTM NFR-1
#[path = "session/support.rs"]
mod support;

use support::*;

use serde_json::json;

// T-12.7: RTM NFR-1 — helper: invoke a tool and return the user-visible text
// regardless of Ok/Err variant (mirrors markdown_boundary_redteam.rs pattern).
fn invoke_text(tool: &str, payload: serde_json::Value) -> String {
    match ToolRegistry::global().try_invoke(tool, &payload, OutputFormat::Markdown) {
        Ok(text) => text,
        Err(err) => err.to_string(),
    }
}

// T-12.7: RTM NFR-1 — helper for MERGE: both read and update of a non-existent URI
// hit the same `find_assumption` lookup path and must return the same error prefix.
fn assert_missing_uri_returns_not_found(action: &str) {
    let mut payload = json!({
        "action": action,
        "uri": "memory://assumptions/2025/01/assumption-nonexistent"
    });
    if action == "update" {
        payload["status"] = json!("validated");
    }
    let result = invoke_text("AssumptionLedger", payload);
    assert!(
        result.contains("Assumption not found"),
        "Expected 'Assumption not found' for action '{action}', got: {result}"
    );
}

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

    // T-12.7: RTM NFR-1 — assert success semantics, not emoji prefix
    assert!(
        !update_result.contains("ERROR"),
        "Update should succeed but got: {update_result}"
    );
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

// T-12.7: RTM NFR-1 — MERGED: both read and update of a missing URI exercise the
// same `find_assumption` lookup path; a single parameterised test covers both.
#[test]
fn non_existent_assumption_returns_error() {
    let _harness = TestHarness::new();
    for action in &["read", "update"] {
        assert_missing_uri_returns_not_found(action);
    }
}

#[test]
fn invalid_action_returns_error() {
    let _harness = TestHarness::new();
    let result = invoke_text("AssumptionLedger", json!({ "action": "invalid-action" }));

    assert!(result.contains("Invalid action"));
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
    let files_before = std::fs::read_dir(&date_dir).ok().map_or(0, Iterator::count);

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

// T-12.7: RTM NFR-1 — REWRITE: assert confidence via frontmatter, not markdown bold syntax
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

    // Verify update succeeded and mentions the new confidence value
    assert!(
        !update_result.contains("ERROR"),
        "Update should succeed but got: {update_result}"
    );
    assert!(
        update_result.contains("high"),
        "Update response should reference new confidence value, got: {update_result}"
    );

    // Verify persistence via frontmatter — the semantic source of truth
    let (frontmatter, _content) = harness.read(&uri);
    assert_eq!(
        frontmatter_string(&frontmatter, "confidence"),
        "high",
        "Confidence should be persisted as 'high' in frontmatter"
    );
}
