#[path = "recent_activity_boundary/support.rs"]
mod support;

use serde_json::json;
use support::*;

// T-2.3: RTM FR-4 - split RecentActivity boundary coverage by rendering and empty-state scenarios.

// T-2.3: RTM FR-4 (TC-4.4 / RTM-025 / RTM-030)
#[test]
fn recent_activity_include_content_includes_h2_heading_and_content() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let conn = open_db(&env.cfg);
    seed_activity(
        &conn,
        "memory://thinking/sequential/2026/03/session-content",
        "Content Session",
        "## Thought 1\nFirst content block\n## Thought 2\nSecond content block",
        &timestamp(1),
        "active",
    );

    let output = invoke_recent_activity(json!({
        "limit": 10,
        "includeContent": true
    }));

    assert!(
        output.contains("First H2: Thought 1"),
        "missing first H2: {output}"
    );
    assert!(
        output.contains("First H2 Content: First content block"),
        "missing first H2 content: {output}"
    );
    assert!(
        output.contains("Last H2: Thought 2"),
        "missing last H2: {output}"
    );
    assert!(
        output.contains("Last H2 Content: Second content block"),
        "missing last H2 content: {output}"
    );
}

// T-2.3: RTM FR-4 (TC-4.6 / RTM-027)
#[test]
fn recent_activity_returns_no_activity_message_when_query_matches_nothing() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    let output = invoke_recent_activity(json!({ "limit": 10 }));

    assert_eq!(output.trim(), "No recent activity found");
}

// T-2.3: RTM FR-4 (TC-4.7 / RTM-028)
#[test]
fn recent_activity_learn_true_returns_help_documentation() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    let output = invoke_recent_activity(json!({ "learn": true }));

    assert!(
        output.contains("RecentActivity"),
        "missing tool name: {output}"
    );
    assert!(
        output.contains("timespan"),
        "missing parameter docs: {output}"
    );
    assert!(
        output.contains("Include full section content") || output.contains("includeContent"),
        "missing content parameter docs: {output}"
    );
    assert!(
        !output.starts_with("# Recent Activity"),
        "learn=true should return help, not live activity output: {output}"
    );
}

// T-2.3: RTM FR-4 (TC-4.8 / RTM-029)
#[test]
fn recent_activity_classifies_sequential_workflow_and_memory_files() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let conn = open_db(&env.cfg);
    seed_activity(
        &conn,
        "memory://thinking/sequential/2026/03/session-classified",
        "Sequential Classified",
        "## Thought 1\nClassified sequential",
        &timestamp(1),
        "active",
    );
    seed_activity(
        &conn,
        "memory://thinking/workflow/2026/03/workflow-classified",
        "Workflow Classified",
        "## Step 1\nClassified workflow",
        &timestamp(2),
        "completed",
    );
    seed_activity(
        &conn,
        "memory://projects/reference/memory-classified",
        "Memory Classified",
        "# Memory Classified\n\n## Intro\nClassified memory",
        &timestamp(3),
        "active",
    );

    let output = invoke_recent_activity(json!({ "limit": 10 }));

    assert!(output.contains("**session-classified** (sequential)"));
    assert!(output.contains("Thoughts: 1"));
    assert!(output.contains("**workflow-classified** (workflow)"));
    assert!(output.contains("Steps: 1"));
    assert!(output.contains("**memory-classified** (memory)"));
    assert!(output.contains("Title: Memory Classified"));
}
