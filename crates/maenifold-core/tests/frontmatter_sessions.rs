#[path = "frontmatter/support.rs"]
mod support;

use maenifold_core::memory;
use maenifold_core::sessions;
use serde_json::json;
use support::{
    TEST_MUTEX, iso_minutes_ago, isolated_env, markdown_context, read_body, read_db_content,
    read_db_status, read_frontmatter_status, seed_file_content, write_raw_markdown,
};

// T-12.6: RTM NFR-1 — Split session/timestamp frontmatter regressions into a focused integration test file.
// T-META-001.6: RTM FR-24.x — Ported frontmatter metadata regression coverage from the C# oracle.

// T-META-001.6: RTM FR-24.6 — Contract test: `modified` frontmatter does NOT override DB-driven cleanup.
// A session with a fresh `modified` frontmatter field but a stale `last_indexed` DB timestamp
// is still abandoned, proving the cleanup path is entirely DB-driven.
#[test]
fn fr24_6_ac1_session_with_fresh_modified_frontmatter_still_abandoned_by_stale_db() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/workflow/stale-with-modified";
    let fresh_modified = iso_minutes_ago(2); // recent frontmatter timestamp — should be ignored
    let body = "# Stale Session\n\nContent";
    let raw_file =
        format!("---\ntype: workflow\nstatus: active\nmodified: {fresh_modified}\n---\n\n{body}");
    write_raw_markdown(&env.cfg, file_uri, &raw_file);
    // DB says 130 min stale — this is what cleanup actually reads
    seed_file_content(&env.cfg, file_uri, "active", body, &iso_minutes_ago(130));

    let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    // T-META-001.6: RTM FR-24.6 — Cleanup relies on DB `last_indexed`, not frontmatter `modified`.
    assert_eq!(abandoned, 1);
    assert_eq!(read_frontmatter_status(&env.cfg, file_uri), "abandoned");
    assert_eq!(read_db_status(&env.cfg, file_uri), "abandoned");
    let body = read_body(&env.cfg, file_uri);
    assert!(
        body.contains("## Session Abandoned"),
        "missing abandon notice in body: {body}"
    );
    let db_content = read_db_content(&env.cfg, file_uri);
    assert!(
        db_content.contains("## Session Abandoned"),
        "db content should be updated too: {db_content}"
    );
}

// T-META-001.6: RTM FR-24.6 — Cleanup of raw-markdown files: fresh stays active, stale is abandoned.
// Exercises `write_raw_markdown`-created files (not via `write_markdown`) through the cleanup
// path, with full assertion depth matching `session_cleanup_ttl.rs`.
#[test]
fn fr24_6_ac2_raw_markdown_session_cleanup_fresh_active_stale_abandoned() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    // --- Fresh raw session: should remain active ---
    let fresh_uri = "memory://thinking/workflow/raw-fresh";
    write_raw_markdown(
        &env.cfg,
        fresh_uri,
        "---\ntype: workflow\nstatus: active\n---\n\n# Session\n\nContent",
    );
    seed_file_content(
        &env.cfg,
        fresh_uri,
        "active",
        "# Session\n\nContent",
        &iso_minutes_ago(5),
    );

    // --- Stale raw session: should be abandoned ---
    let stale_uri = "memory://thinking/sequential/raw-stale";
    write_raw_markdown(
        &env.cfg,
        stale_uri,
        "---\ntype: sequential\nstatus: active\n---\n\n# Old Session\n\nContent",
    );
    seed_file_content(
        &env.cfg,
        stale_uri,
        "active",
        "# Old Session\n\nContent",
        &iso_minutes_ago(150),
    );

    let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    // T-META-001.6: RTM FR-24.6 — Only the stale session is abandoned.
    assert_eq!(abandoned, 1);

    // Fresh session: unchanged
    assert_eq!(read_frontmatter_status(&env.cfg, fresh_uri), "active");
    assert_eq!(read_db_status(&env.cfg, fresh_uri), "active");
    let fresh_body = read_body(&env.cfg, fresh_uri);
    assert!(
        !fresh_body.contains("## Session Abandoned"),
        "fresh session should not be abandoned: {fresh_body}"
    );

    // Stale session: abandoned with full depth
    assert_eq!(read_frontmatter_status(&env.cfg, stale_uri), "abandoned");
    assert_eq!(read_db_status(&env.cfg, stale_uri), "abandoned");
    let stale_body = read_body(&env.cfg, stale_uri);
    assert!(
        stale_body.contains("## Session Abandoned"),
        "missing abandon notice in body: {stale_body}"
    );
    let db_content = read_db_content(&env.cfg, stale_uri);
    assert!(
        db_content.contains("## Session Abandoned"),
        "db content should be updated too: {db_content}"
    );
}

// T-META-001.6: RTM FR-24.6 — ReadMemory renders "N/A" when created/modified timestamps are missing.
#[test]
fn fr24_6_ac4_read_memory_renders_na_when_timestamps_missing() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://fr24-no-fm/no-timestamps";
    write_raw_markdown(
        &env.cfg,
        file_uri,
        "---\ntype: memory\ntitle: No Timestamps\n---\n\n# No Timestamps\n\nContent with [[raw-file]] that has no timestamp frontmatter.",
    );

    let output = memory::read_memory(
        &markdown_context(&env.cfg),
        &json!({
            "identifier": file_uri,
            "includeChecksum": true
        }),
    )
    .expect("read memory");

    // T-META-001.6: RTM FR-24.6 — Falsifiable assertions: "N/A" is the expected fallback.
    assert!(!output.starts_with("ERROR:"), "unexpected output: {output}");
    assert!(
        output.contains("Created: N/A"),
        "expected 'Created: N/A' but got: {output}"
    );
    assert!(
        output.contains("Modified: N/A"),
        "expected 'Modified: N/A' but got: {output}"
    );
}
