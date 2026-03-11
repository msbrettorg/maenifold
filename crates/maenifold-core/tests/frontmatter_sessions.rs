#[path = "frontmatter/support.rs"]
mod support;

use maenifold_core::memory;
use maenifold_core::sessions;
use serde_json::json;
use support::{
    TEST_MUTEX, iso_minutes_ago, isolated_env, markdown_context, read_frontmatter_status,
    seed_file_content, write_raw_markdown,
};

// T-12.6: RTM NFR-1 — Split session/timestamp frontmatter regressions into a focused integration test file.
// T-META-001.6: RTM FR-24.x — Ported frontmatter metadata regression coverage from the C# oracle.

// T-META-001.6: RTM FR-24.6 — Session cleanup coverage for timestamp scenarios.
#[test]
fn fr24_6_ac1_session_cleanup_with_modified_existing_behavior() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/workflow/stale-with-modified";
    let stale_modified = iso_minutes_ago(130);
    let body = "# Stale Session\n\nContent";
    let raw_file =
        format!("---\ntype: workflow\nstatus: active\nmodified: {stale_modified}\n---\n\n{body}");
    write_raw_markdown(&env.cfg, file_uri, &raw_file);
    seed_file_content(&env.cfg, file_uri, "active", body, &iso_minutes_ago(130));

    let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    assert_eq!(abandoned, 1);
    assert_eq!(read_frontmatter_status(&env.cfg, file_uri), "abandoned");
}

// T-META-001.6: RTM FR-24.6 — Recent activity remains active when cleanup input is fresh.
#[test]
fn fr24_6_ac2_session_cleanup_without_modified_filesystem_fallback() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/workflow/no-modified-recent";
    let body = "# Session\n\nContent";
    write_raw_markdown(
        &env.cfg,
        file_uri,
        "---\ntype: workflow\nstatus: active\n---\n\n# Session\n\nContent",
    );
    seed_file_content(&env.cfg, file_uri, "active", body, &iso_minutes_ago(5));

    let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    assert_eq!(abandoned, 0);
    assert_eq!(read_frontmatter_status(&env.cfg, file_uri), "active");
}

// T-META-001.6: RTM FR-24.6 — Stale active sessions are abandoned even without modified frontmatter.
#[test]
fn fr24_6_ac3_session_cleanup_abandoned_detected_regardless_of_frontmatter() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/sequential/old-no-modified";
    let body = "# Old Session\n\nContent";
    write_raw_markdown(
        &env.cfg,
        file_uri,
        "---\ntype: sequential\nstatus: active\n---\n\n# Old Session\n\nContent",
    );
    seed_file_content(&env.cfg, file_uri, "active", body, &iso_minutes_ago(150));

    let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    assert_eq!(abandoned, 1);
    assert_eq!(read_frontmatter_status(&env.cfg, file_uri), "abandoned");
}

// T-META-001.6: RTM FR-24.6 — ReadMemory still renders Created/Modified lines when timestamps are missing.
#[test]
fn fr24_6_ac4_read_memory_displays_filesystem_timestamps_when_frontmatter_missing() {
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

    assert!(!output.starts_with("ERROR:"), "unexpected output: {output}");
    assert!(
        output.contains("Created:"),
        "missing Created line: {output}"
    );
    assert!(
        output.contains("Modified:"),
        "missing Modified line: {output}"
    );
}
