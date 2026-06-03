//! Regression test for T-243.32: list_memories must not block or fail when the
//! memory vault contains OneDrive dataless (.md) files.
//!
//! A dataless file carries the `com.apple.decmpfs` extended attribute.
//! `fs::read_to_string` on such a file blocks until the cloud provider
//! materialises the content — potentially exceeding the 60 s MCP timeout.
//!
//! Fix: `read_created_date` detects the xattr and uses filesystem timestamps
//! instead.  `list_memories` continues with degraded metadata on any per-file
//! error so the whole listing is never blocked by a single bad file.

#[cfg(target_os = "macos")]
use maenifold_core::util::is_dataless;
use maenifold_core::{
    config::Config, memory::list_memories, tools::ToolContext, util::OutputFormat,
};
use std::io::Write;
#[cfg(target_os = "macos")]
use std::process::Command;
use std::time::{Duration, Instant};

fn make_test_config(root: &std::path::Path) -> Config {
    Config::override_root(root);
    let cfg = Config::current();
    std::fs::create_dir_all(&cfg.memory_path).expect("create memory dir");
    cfg
}

/// Create a plain .md file and return its path.
fn write_md(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    let mut f = std::fs::File::create(&path).expect("create md");
    f.write_all(content.as_bytes()).expect("write md");
    path
}

/// Apply the com.apple.decmpfs xattr to simulate an OneDrive dataless file.
/// Uses the `xattr` CLI (macOS built-in) so we exercise the real kernel path.
#[cfg(target_os = "macos")]
fn mark_dataless(path: &std::path::Path) {
    let status = Command::new("xattr")
        .args(["-wx", "com.apple.decmpfs", "66706d63"])
        .arg(path)
        .status()
        .expect("xattr command");
    assert!(status.success(), "xattr -wx failed for {}", path.display());
}

// ── Test 1: is_dataless correctly identifies the attribute ──────────────────

#[test]
#[cfg(target_os = "macos")]
fn test_is_dataless_detects_xattr() {
    let dir = tempfile::tempdir().expect("tempdir");
    let normal = write_md(dir.path(), "normal.md", "# Normal\n");
    let flagged = write_md(dir.path(), "flagged.md", "# Flagged\n");
    mark_dataless(&flagged);

    assert!(!is_dataless(&normal), "normal file must not be dataless");
    assert!(is_dataless(&flagged), "flagged file must be dataless");
}

// ── Test 2: list_memories returns within 1 s and includes the dataless file ─

#[test]
#[cfg(target_os = "macos")]
fn test_list_memories_dataless_no_block() {
    let dir = tempfile::tempdir().expect("tempdir");
    // Root must be $dir; memory lives at $dir/memory
    let memory_dir = dir.path().join("memory");
    std::fs::create_dir_all(&memory_dir).expect("memory dir");

    // Normal file
    let _ = write_md(
        &memory_dir,
        "normal.md",
        "---\ncreated: 2024-01-01 00:00:00\n---\nNormal content\n",
    );
    // Dataless file — xattr set, but content is unreadable in production
    let dataless = write_md(&memory_dir, "AGENTS.md", "# Agents\n");
    mark_dataless(&dataless);

    let cfg = make_test_config(dir.path());
    let context = ToolContext::new(cfg, OutputFormat::Markdown);

    let start = Instant::now();
    let result = list_memories(&context, &serde_json::json!({}));
    let elapsed = start.elapsed();

    // (a) returns within 1 s
    assert!(
        elapsed < Duration::from_secs(1),
        "list_memories took {:?} — must complete within 1 s",
        elapsed
    );

    // (b) does not panic (reaching here proves it)
    let output = result.expect("list_memories must not return Err");

    // (c) both files appear in the listing
    assert!(
        output.contains("normal.md"),
        "normal.md must appear in listing; got:\n{output}"
    );
    assert!(
        output.contains("AGENTS.md"),
        "AGENTS.md (dataless) must appear in listing; got:\n{output}"
    );
}

// ── Test 3: non-macOS smoke test — list_memories on a normal vault ───────────

#[test]
fn test_list_memories_normal_vault() {
    let dir = tempfile::tempdir().expect("tempdir");
    let memory_dir = dir.path().join("memory");
    std::fs::create_dir_all(&memory_dir).expect("memory dir");
    let _ = write_md(
        &memory_dir,
        "note.md",
        "---\ncreated: 2024-06-01 00:00:00\n---\nContent\n",
    );

    let cfg = make_test_config(dir.path());
    let context = ToolContext::new(cfg, OutputFormat::Markdown);

    let start = Instant::now();
    let result = list_memories(&context, &serde_json::json!({}));
    let elapsed = start.elapsed();

    assert!(elapsed < Duration::from_secs(1), "took {:?}", elapsed);
    let output = result.expect("must succeed");
    assert!(
        output.contains("note.md"),
        "note.md must appear; got:\n{output}"
    );
}
