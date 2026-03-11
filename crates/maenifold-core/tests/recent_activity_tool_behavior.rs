//! Ported from C# RecentActivityTests.cs — RecentActivity behavior coverage.
//! T-2.3: RTM FR-4 — RecentActivity tool boundary + integration tests.

#[path = "recent_activity_tool/support.rs"]
mod support;

// T-2.3: RTM FR-4 — behavior-focused split from recent_activity_tool_tests.rs.
use maenifold_core::ToolRegistry;
use maenifold_core::json::OutputFormat;
use serde_json::json;
use support::{
    TEST_MUTEX, invoke_tool, isolated_env, rendered_item_count, seed_chat_file, seed_memory_file,
    seed_sequential_file, seed_workflow_file, set_last_indexed,
};

#[test]
fn recent_activity_filters_timespan_include_content_workflow_and_sequential() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    // T-2.3: RTM FR-4 / seed plain memory article
    seed_memory_file(
        &env.cfg,
        "article-alpha.md",
        "# Article Alpha\n\n## Intro\nMemory body A\n\n## More\nMemory body B\n",
    );

    // T-2.3: RTM FR-4 / seed sequential thinking session
    seed_sequential_file(
        &env.cfg,
        "session-1",
        "## Thought 1/2\nFirst thought\n\n## Thought 2/2\nSecond thought",
    );

    // T-2.3: RTM FR-4 / seed workflow thinking session
    seed_workflow_file(
        &env.cfg,
        "workflow-1",
        "## Step 1\nStep one response\n\n## Step 2\nStep two response",
    );

    // T-2.3: RTM FR-4 / seed chat-assumption file
    seed_chat_file(
        &env.cfg,
        "chat-1.md",
        "# Chat Log\n\n## Message\nHello from chat\n",
    );

    let sync_output = invoke_tool("Sync", json!({}));
    assert!(
        sync_output.contains("Sync"),
        "Sync should run successfully: {sync_output}"
    );
    assert!(
        env.cfg.database_path.exists(),
        "Database should be created by Sync"
    );

    set_last_indexed(&env.cfg, "article-alpha", "2020-01-01 00:00:00");

    let all_out = invoke_tool("RecentActivity", json!({ "limit": 10 }));
    assert!(
        all_out.contains("# Recent Activity"),
        "all: missing header: {all_out}"
    );
    assert!(
        all_out.contains("(sequential)"),
        "all: missing sequential: {all_out}"
    );
    assert!(
        all_out.contains("(workflow)"),
        "all: missing workflow: {all_out}"
    );
    assert!(
        all_out.contains("(memory)"),
        "all: missing memory: {all_out}"
    );

    let think_out = invoke_tool(
        "RecentActivity",
        json!({ "limit": 10, "filter": "thinking" }),
    );
    assert!(
        think_out.contains("(sequential)"),
        "thinking: missing sequential: {think_out}"
    );
    assert!(
        think_out.contains("(workflow)"),
        "thinking: missing workflow: {think_out}"
    );
    assert!(
        !think_out.contains("(memory)"),
        "thinking: should not contain memory: {think_out}"
    );

    let mem_out = invoke_tool("RecentActivity", json!({ "limit": 10, "filter": "memory" }));
    assert!(
        mem_out.contains("(memory)"),
        "memory: missing memory: {mem_out}"
    );
    assert!(
        !mem_out.contains("(sequential)"),
        "memory: should not contain sequential: {mem_out}"
    );
    assert!(
        !mem_out.contains("(workflow)"),
        "memory: should not contain workflow: {mem_out}"
    );

    let chat_out = invoke_tool(
        "RecentActivity",
        json!({ "limit": 10, "filter": "assumptions" }),
    );
    assert!(
        chat_out.contains("(memory)"),
        "assumptions: entries render as memory type: {chat_out}"
    );
    assert!(
        chat_out.contains("Title: chat-1") || chat_out.contains("Title: Chat Log"),
        "assumptions: expected chat title: {chat_out}"
    );
    assert!(
        !chat_out.contains("(sequential)"),
        "assumptions: should not contain sequential: {chat_out}"
    );
    assert!(
        !chat_out.contains("(workflow)"),
        "assumptions: should not contain workflow: {chat_out}"
    );

    let include_out = invoke_tool(
        "RecentActivity",
        json!({ "limit": 10, "includeContent": true }),
    );
    assert!(
        include_out.contains("First H2:"),
        "includeContent: missing First H2: {include_out}"
    );
    assert!(
        include_out.contains("First H2 Content:"),
        "includeContent: missing First H2 Content: {include_out}"
    );
    assert!(
        include_out.contains("Thought 1/2"),
        "includeContent: missing sequential H2 heading: {include_out}"
    );
    assert!(
        include_out.contains("Step 1"),
        "includeContent: missing workflow H2 heading: {include_out}"
    );

    let recent_only = invoke_tool(
        "RecentActivity",
        json!({ "limit": 10, "timespan": "1.00:00:00" }),
    );
    assert!(
        !recent_only.contains("Article Alpha"),
        "timespan: old article should be excluded: {recent_only}"
    );
    assert!(
        recent_only.contains("(sequential)"),
        "timespan: recent sequential should remain: {recent_only}"
    );
    assert!(
        recent_only.contains("(workflow)"),
        "timespan: recent workflow should remain: {recent_only}"
    );

    let bad = ToolRegistry::global().try_invoke(
        "RecentActivity",
        &json!({ "timespan": "-1.00:00:00" }),
        OutputFormat::Markdown,
    );
    let err = bad.expect_err("negative timespan must error");
    let message = err.to_string();
    assert!(
        message.contains("timespan must be positive"),
        "negative timespan should report validation failure: {message}"
    );

    let one = invoke_tool("RecentActivity", json!({ "limit": 1 }));
    let count = rendered_item_count(&one);
    assert_eq!(
        count, 1,
        "limit=1 should return a single activity item, got {count}: {one}"
    );
}
