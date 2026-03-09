//! Ported from C# RecentActivityTests.cs — end-to-end tests for RecentActivity tool.
//!
//! Two tests:
//! 1. `recent_activity_filters_timespan_include_content_workflow_and_sequential` — multi-step
//!    end-to-end covering filters, timespan, includeContent, limit, and negative timespan.
//! 2. `recent_activity_performance_compared_to_file_scan` — performance measurement (#[ignore]).
//!
//! T-2.3: RTM FR-4 — RecentActivity tool boundary + integration tests

use chrono::Utc;
use maenifold_core::json::OutputFormat;
use maenifold_core::{AssetManager, Config, GraphDatabase, ToolRegistry};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::Connection;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

struct TestEnv {
    _tmp: TempDir,
    cfg: Config,
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

fn isolated_env() -> TestEnv {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    AssetManager::initialize_assets(&cfg).expect("assets");
    GraphDatabase::initialize_database(&cfg).expect("db init");
    TestEnv { _tmp: tmp, cfg }
}

fn open_db(cfg: &Config) -> Connection {
    Connection::open(&cfg.database_path).expect("open db")
}

fn invoke_tool(tool: &str, payload: serde_json::Value) -> String {
    ToolRegistry::global()
        .try_invoke(tool, &payload, OutputFormat::Markdown)
        .expect(&format!("invoke {tool}"))
}

fn seed_memory_file(cfg: &Config, relative_path: &str, content: &str) {
    let full_path = cfg.memory_path.join(relative_path);
    fs::create_dir_all(full_path.parent().expect("parent")).expect("memory dir");
    fs::write(full_path, content).expect("memory file");
}

fn seed_sequential_file(cfg: &Config, session_id: &str, thought_content: &str) {
    let seq_dir = cfg
        .sequential_path
        .join(Utc::now().format("%Y").to_string())
        .join(Utc::now().format("%m").to_string());
    fs::create_dir_all(&seq_dir).expect("sequential dir");
    let content = format!("# {session_id}\n\n{thought_content}\n");
    fs::write(seq_dir.join(format!("{session_id}.md")), content).expect("sequential file");
}

fn seed_workflow_file(cfg: &Config, workflow_id: &str, step_content: &str) {
    let wf_dir = cfg
        .thinking_path
        .join("workflow")
        .join(Utc::now().format("%Y").to_string())
        .join(Utc::now().format("%m").to_string());
    fs::create_dir_all(&wf_dir).expect("workflow dir");
    let content = format!("# {workflow_id}\n\n{step_content}\n");
    fs::write(wf_dir.join(format!("{workflow_id}.md")), content).expect("workflow file");
}

fn seed_chat_file(cfg: &Config, file_name: &str, content: &str) {
    let chat_dir = cfg
        .memory_path
        .join("chat")
        .join(Utc::now().format("%Y").to_string())
        .join(Utc::now().format("%m").to_string());
    fs::create_dir_all(&chat_dir).expect("chat dir");
    fs::write(chat_dir.join(file_name), content).expect("chat file");
}

fn set_last_indexed(cfg: &Config, file_name_fragment: &str, timestamp_utc: &str) {
    let fragment = file_name_fragment
        .strip_suffix(".md")
        .unwrap_or(file_name_fragment);
    let conn = open_db(cfg);
    let rows = conn
        .execute(
            "UPDATE file_content SET last_indexed = ?1 WHERE file_path LIKE ?2",
            rusqlite::params![timestamp_utc, format!("%{fragment}%")],
        )
        .expect("update last_indexed");
    assert!(
        rows >= 1,
        "expected to update at least one row matching '{fragment}', updated {rows}"
    );
}

fn rendered_item_count(output: &str) -> usize {
    output
        .lines()
        .filter(|line| line.trim_start().starts_with("**"))
        .count()
}

// ---------------------------------------------------------------------------
// T-2.3: RTM FR-4 — Ported from C# RecentActivityFiltersTimespanIncludeContentWorkflowAndSequential
// ---------------------------------------------------------------------------
// This is the comprehensive multi-step end-to-end test that:
//   1. Seeds memory, sequential, workflow, and chat files
//   2. Syncs to populate DB
//   3. Makes one article appear old via direct DB timestamp manipulation
//   4. Verifies filter=all, filter=thinking, filter=memory, filter=assumptions
//   5. Verifies includeContent=true output
//   6. Verifies timespan filter excludes old entries
//   7. Verifies negative timespan errors
//   8. Verifies limit=1 returns a single item
#[test]
fn recent_activity_filters_timespan_include_content_workflow_and_sequential() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    // --- Seed files (mirrors C# test layout) ---

    // T-2.3: RTM FR-4 / plain memory article
    seed_memory_file(
        &env.cfg,
        "article-alpha.md",
        "# Article Alpha\n\n## Intro\nMemory body A\n\n## More\nMemory body B\n",
    );

    // T-2.3: RTM FR-4 / sequential thinking session
    seed_sequential_file(
        &env.cfg,
        "session-1",
        "## Thought 1/2\nFirst thought\n\n## Thought 2/2\nSecond thought",
    );

    // T-2.3: RTM FR-4 / workflow thinking session
    seed_workflow_file(
        &env.cfg,
        "workflow-1",
        "## Step 1\nStep one response\n\n## Step 2\nStep two response",
    );

    // T-2.3: RTM FR-4 / chat/assumption file
    seed_chat_file(
        &env.cfg,
        "chat-1.md",
        "# Chat Log\n\n## Message\nHello from chat\n",
    );

    // --- Sync to populate DB ---
    let sync_output = invoke_tool("Sync", json!({}));
    assert!(
        sync_output.contains("Sync"),
        "Sync should run successfully: {sync_output}"
    );
    assert!(
        env.cfg.database_path.exists(),
        "Database should be created by Sync"
    );

    // Make article-alpha appear old (2020) to test timespan cutoff
    set_last_indexed(&env.cfg, "article-alpha", "2020-01-01 00:00:00");

    // 1) all (default) — should contain sequential, workflow, memory items
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

    // 2) filter=thinking → sequential/workflow only
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

    // 3) filter=memory → exclude thinking and chat
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

    // 4) filter=assumptions → assumption/chat entries only (rendered as memory type)
    let chat_out = invoke_tool(
        "RecentActivity",
        json!({ "limit": 10, "filter": "assumptions" }),
    );
    assert!(
        chat_out.contains("(memory)"),
        "assumptions: entries render as memory type: {chat_out}"
    );
    // The C# test checks for "Title: chat-1" (filename). The Rust implementation
    // renders the title field from the DB which is the filename stem when Sync indexes.
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

    // 5) includeContent=true → show explicit H2 headings and content
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

    // 6) timespan filter should exclude the old article (set to 2020)
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

    // 7) negative timespan → error
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

    // 8) limit=1 should only show a single entry block
    let one = invoke_tool("RecentActivity", json!({ "limit": 1 }));
    let count = rendered_item_count(&one);
    assert_eq!(
        count, 1,
        "limit=1 should return a single activity item, got {count}: {one}"
    );
}

// ---------------------------------------------------------------------------
// T-2.3: RTM FR-4 — Ported from C# RecentActivityPerformanceComparedToFileScan
// ---------------------------------------------------------------------------
// Performance measurement test — generates 300 files, syncs, then compares
// DB-backed RecentActivity invocation time vs. a naive file-system scan.
// Marked #[ignore] since it is a manual performance test (C# has [Explicit]).
#[test]
#[ignore]
fn recent_activity_performance_compared_to_file_scan() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    // Generate 300 files: 150 sequential + 150 memory docs
    let seq_dir = env
        .cfg
        .sequential_path
        .join(Utc::now().format("%Y").to_string())
        .join(Utc::now().format("%m").to_string());
    let docs_dir = env.cfg.memory_path.join("docs");
    fs::create_dir_all(&seq_dir).expect("seq dir");
    fs::create_dir_all(&docs_dir).expect("docs dir");

    for i in 0..150 {
        let seq_content =
            format!("# S{i}\n\n## Thought 1/2\nHello {i}\n\n## Thought 2/2\nWorld {i}\n");
        fs::write(seq_dir.join(format!("session-{i:03}.md")), seq_content).expect("write seq");

        let mem_content = format!("# A{i}\n\n## H2-1\nAlpha {i}\n\n## H2-2\nBeta {i}\n");
        fs::write(docs_dir.join(format!("article-{i:03}.md")), mem_content).expect("write mem");
    }

    // Sync all files into DB
    invoke_tool("Sync", json!({}));

    // Measure DB-backed RecentActivity
    let db_start = std::time::Instant::now();
    let _db_output = invoke_tool("RecentActivity", json!({ "limit": 200 }));
    let db_elapsed = db_start.elapsed();

    // Measure naive file scan with H2 extraction (mirrors C# ScanFilesForH2)
    let scan_start = std::time::Instant::now();
    let scan_count = scan_files_for_h2(&env.cfg.memory_path);
    let scan_elapsed = scan_start.elapsed();

    // T-2.3: RTM FR-4 — report performance (mirroring C# Assert.Pass with message)
    println!(
        "Perf measured — DB RecentActivity: {} ms, File scan (count={}): {} ms",
        db_elapsed.as_millis(),
        scan_count,
        scan_elapsed.as_millis()
    );
}

/// Naive file-system H2 extraction scan — mirrors the C# helper `ScanFilesForH2`.
fn scan_files_for_h2(root: &std::path::Path) -> usize {
    let mut file_count = 0usize;
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let text = match fs::read_to_string(path) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let mut current_h2: Option<String> = None;
        let mut sections: Vec<(String, String)> = Vec::new();
        let mut buf = String::new();
        for line in text.lines() {
            if let Some(heading) = line.strip_prefix("## ") {
                if let Some(h2) = current_h2.take() {
                    sections.push((h2, buf.trim().to_string()));
                    buf.clear();
                }
                current_h2 = Some(heading.trim().to_string());
            } else if !line.starts_with("# ") {
                buf.push_str(line);
                buf.push('\n');
            }
        }
        if let Some(h2) = current_h2.take() {
            sections.push((h2, buf.trim().to_string()));
        }
        if !sections.is_empty() {
            file_count += 1;
        }
    }
    file_count
}
