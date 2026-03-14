//! T-2.3: RTM FR-4 — RecentActivity full-pipeline integration smoke tests.
//!
//! This file verifies that all four file types (memory, sequential, workflow, chat)
//! survive the complete seed → Sync → RecentActivity pipeline together. Filter,
//! timespan, includeContent, and negative-timespan behaviors are covered with
//! stronger assertions in the split test suite (boundary_queries, boundary_rendering,
//! pipeline_collect, pipeline_format, pipeline_helpers, pipeline_tool).

#[path = "recent_activity_tool/support.rs"]
mod support;

use serde_json::json;
use support::{
    TEST_MUTEX, invoke_tool, isolated_env, rendered_item_count, seed_chat_file, seed_memory_file,
    seed_sequential_file, seed_workflow_file,
};

// T-2.3: RTM FR-4 — all four file types through one Sync run.
#[test]
fn recent_activity_all_file_types_survive_full_sync_pipeline() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    // T-2.3.1: RTM FR-4 — seed one of each file type
    seed_memory_file(
        &env.cfg,
        "article-alpha.md",
        "# Article Alpha\n\n## Intro\nMemory body A\n\n## More\nMemory body B\n",
    );
    seed_sequential_file(
        &env.cfg,
        "session-1",
        "## Thought 1/2\nFirst thought\n\n## Thought 2/2\nSecond thought",
    );
    seed_workflow_file(
        &env.cfg,
        "workflow-1",
        "## Step 1\nStep one response\n\n## Step 2\nStep two response",
    );
    seed_chat_file(
        &env.cfg,
        "chat-1.md",
        "# Chat Log\n\n## Message\nHello from chat\n",
    );

    invoke_tool("Sync", json!({}));

    let out = invoke_tool("RecentActivity", json!({ "limit": 10 }));
    assert!(
        out.contains("(sequential)"),
        "sequential missing from pipeline output: {out}"
    );
    assert!(
        out.contains("(workflow)"),
        "workflow missing from pipeline output: {out}"
    );
    assert!(
        out.contains("(memory)"),
        "memory missing from pipeline output: {out}"
    );
}

// T-2.3.2: RTM FR-4 / RTM-026 — limit=1 boundary (unique edge case).
// Existing limit tests use limit=2; this covers the boundary between
// "at least one result" and "empty result".
#[test]
fn recent_activity_limit_one_returns_exactly_one_item() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    // Seed multiple items so limit is actually constraining
    seed_memory_file(
        &env.cfg,
        "note-a.md",
        "# Note A\n\n## Section\nBody A\n",
    );
    seed_memory_file(
        &env.cfg,
        "note-b.md",
        "# Note B\n\n## Section\nBody B\n",
    );
    seed_sequential_file(
        &env.cfg,
        "session-limit",
        "## Thought 1/1\nOnly thought",
    );

    invoke_tool("Sync", json!({}));

    let out = invoke_tool("RecentActivity", json!({ "limit": 1 }));
    let count = rendered_item_count(&out);
    assert_eq!(
        count, 1,
        "limit=1 should return exactly one activity item, got {count}: {out}"
    );
}
