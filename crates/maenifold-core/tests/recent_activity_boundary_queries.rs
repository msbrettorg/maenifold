#[allow(clippy::duplicate_mod)]
#[path = "recent_activity_boundary/support.rs"]
mod support;

use serde_json::json;
use support::*;

// T-2.3: RTM FR-4 - split RecentActivity boundary coverage by query-oriented scenarios.

// T-2.3: RTM FR-4 (TC-4.1 / RTM-020)
#[test]
fn recent_activity_default_query_orders_by_last_indexed_desc() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let conn = open_db(&env.cfg);
    seed_mixed_activity(&conn);

    let output = invoke_recent_activity(json!({ "limit": 10 }));

    assert!(
        output.starts_with("# Recent Activity"),
        "unexpected output: {output}"
    );
    let sequential_idx = output.find("**session-recent**").expect("sequential item");
    let workflow_idx = output.find("**workflow-middle**").expect("workflow item");
    let memory_idx = output.find("**knowledge-note**").expect("memory item");
    let assumptions_idx = output.find("**assumption-log**").expect("assumption item");
    assert!(
        sequential_idx < workflow_idx && workflow_idx < memory_idx && memory_idx < assumptions_idx,
        "items should be ordered by last_indexed desc: {output}"
    );
}

// T-2.3: RTM FR-4 (TC-4.2 / RTM-021 / RTM-022 / RTM-023)
#[test]
fn recent_activity_filter_variants_select_expected_path_families() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let conn = open_db(&env.cfg);
    seed_mixed_activity(&conn);

    let thinking = invoke_recent_activity(json!({ "limit": 10, "filter": "thinking" }));
    assert!(thinking.contains("**session-recent** (sequential)"));
    assert!(thinking.contains("**workflow-middle** (workflow)"));
    assert!(!thinking.contains("**knowledge-note**"));
    assert!(!thinking.contains("**assumption-log**"));

    let memory = invoke_recent_activity(json!({ "limit": 10, "filter": "memory" }));
    assert!(memory.contains("**knowledge-note** (memory)"));
    assert!(!memory.contains("**session-recent**"));
    assert!(!memory.contains("**workflow-middle**"));
    assert!(!memory.contains("**assumption-log**"));

    let assumptions = invoke_recent_activity(json!({ "limit": 10, "filter": "assumptions" }));
    assert!(assumptions.contains("**assumption-log** (memory)"));
    assert!(assumptions.contains("Title: Assumption Log"));
    assert!(!assumptions.contains("**session-recent**"));
    assert!(!assumptions.contains("**workflow-middle**"));
    assert!(!assumptions.contains("**knowledge-note**"));
}

// T-2.3: RTM FR-4 (TC-4.3 / RTM-024)
#[test]
fn recent_activity_timespan_restricts_results_to_requested_window() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let conn = open_db(&env.cfg);
    seed_activity(
        &conn,
        "memory://thinking/sequential/2026/03/session-fresh",
        "Fresh Session",
        "## Thought 1\nFresh activity",
        &timestamp(2),
        "active",
    );
    seed_activity(
        &conn,
        "memory://projects/archive/session-stale",
        "Stale Memory",
        "# Stale\n\n## Intro\nOut of window",
        &timestamp_days_ago(3),
        "active",
    );

    let output = invoke_recent_activity(json!({
        "limit": 10,
        "timespan": "24.00:00:00"
    }));

    assert!(
        output.contains("**session-fresh**"),
        "expected fresh row: {output}"
    );
    assert!(
        !output.contains("**session-stale**"),
        "stale row should be excluded by timespan: {output}"
    );
}

// T-2.3: RTM FR-4 (TC-4.5 / RTM-026)
#[test]
fn recent_activity_limit_caps_number_of_results() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let conn = open_db(&env.cfg);
    seed_mixed_activity(&conn);

    let output = invoke_recent_activity(json!({ "limit": 2 }));

    assert_eq!(
        rendered_item_count(&output),
        2,
        "unexpected output: {output}"
    );
    assert!(output.contains("**session-recent**"));
    assert!(output.contains("**workflow-middle**"));
    assert!(!output.contains("**knowledge-note**"));
    assert!(!output.contains("**assumption-log**"));
}
