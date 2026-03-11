// T-12.7: RTM NFR-1
#[path = "workflow_supervisor/support.rs"]
mod support;

use std::fs;

use maenifold_core::markdown::Frontmatter;
use maenifold_core::sessions;
use serde_yaml::Value as YamlValue;
use support::*;

// T-W1-7: RTM FR-17.7
#[test]
fn handle_session_cleanup_memory_type_no_change() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://notes/memory-type-test";
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert("type".to_string(), YamlValue::String("memory".to_string()));
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("active".to_string()),
    );
    write_raw_markdown(&env.cfg, file_uri, &frontmatter, "# Memory Note\n\nContent");
    seed_file_content(
        &env.cfg,
        file_uri,
        Some("active"),
        "# Memory Note\n\nContent",
        &iso_minutes_ago(130),
    );
    let path = path_for_uri(&env.cfg, file_uri);
    let before_mtime = fs::metadata(&path)
        .expect("metadata")
        .modified()
        .expect("mtime");

    let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    let after_mtime = fs::metadata(&path)
        .expect("metadata")
        .modified()
        .expect("mtime");
    assert_eq!(abandoned, 0);
    assert_eq!(before_mtime, after_mtime);
}

// T-W1-7: RTM FR-17.7
#[test]
fn handle_session_cleanup_already_abandoned_no_change() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/workflow/2026/03/09/already-abandoned-test";
    create_session(
        &env.cfg,
        file_uri,
        "abandoned",
        130,
        "# Abandoned Session\n\nContent [[Session Cleanup]]",
    );
    let before_body = read_body(&env.cfg, file_uri);

    let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    assert_eq!(abandoned, 0);
    assert_eq!(read_frontmatter_status(&env.cfg, file_uri), "abandoned");
    assert_eq!(read_body(&env.cfg, file_uri), before_body);
}
