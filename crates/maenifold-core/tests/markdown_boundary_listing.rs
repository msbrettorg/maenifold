#[path = "markdown_boundary/support.rs"]
mod support;

use maenifold_core::MaenifoldError;
use serde_json::json;
use std::fs;
use support::*;

// T-12.7: RTM NFR-1 - split markdown listing boundary coverage by theme.

// T-W1-2: RTM FR-28.5
#[test]
fn fr28_5_ac1_get_memory_files_returns_memory_uris() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_file(&env.cfg, "memory://alpha", None, "# Test\n\nContent");
    seed_file(&env.cfg, "memory://beta", None, "# Test\n\nContent");
    seed_file(&env.cfg, "memory://sub/gamma", None, "# Test\n\nContent");

    let files = markdown::get_memory_files(&env.cfg, None).expect("memory files");

    assert_eq!(files.len(), 3);
    assert!(files.iter().all(|file| file.starts_with("memory://")));
    assert!(files.contains(&"memory://alpha".to_string()));
    assert!(files.contains(&"memory://beta".to_string()));
    assert!(files.contains(&"memory://sub/gamma".to_string()));
}

// T-W1-2: RTM FR-28.5
#[test]
fn fr28_5_ac2_get_memory_files_excludes_hidden_files() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_file(&env.cfg, "memory://visible-note", None, "# Visible");
    let hidden_dir = env.cfg.memory_path.join(".obsidian");
    fs::create_dir_all(&hidden_dir).expect("hidden dir");
    fs::write(hidden_dir.join("hidden-note.md"), "# Hidden").expect("hidden note");

    let files = markdown::get_memory_files(&env.cfg, None).expect("memory files");

    assert!(files.iter().all(|file| file.starts_with("memory://")));
    assert!(files.contains(&"memory://visible-note".to_string()));
    assert!(files.iter().all(|file| !file.contains(".obsidian")));
}

// T-W1-2: RTM FR-28.5
#[test]
fn fr28_5_ac3_get_memory_files_folder_uri_filter_returns_only_subset() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_file(&env.cfg, "memory://docs/readme", None, "# Test");
    seed_file(&env.cfg, "memory://docs/guide", None, "# Test");
    seed_file(&env.cfg, "memory://notes/daily", None, "# Test");

    let files = markdown::get_memory_files(&env.cfg, Some("memory://docs")).expect("docs files");

    assert_eq!(files.len(), 2);
    assert!(files.iter().all(|file| file.starts_with("memory://docs/")));
    assert!(
        files
            .iter()
            .all(|file| !file.starts_with("memory://notes/"))
    );
}

// T-W1-2: RTM FR-28.6
#[test]
fn fr28_6_ac1_list_memories_returns_directories() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_file(&env.cfg, "memory://folder-a/note", None, "# Test");
    seed_file(&env.cfg, "memory://folder-b/note", None, "# Test");

    let output = memory::list_memories(&env.ctx(), &json!({})).expect("list memories");

    // T-W1-2: RTM FR-28.6 - verify production list_memories surfaces seeded directories.
    assert!(
        output.contains("folder-a"),
        "expected folder-a in output: {output}"
    );
    assert!(
        output.contains("folder-b"),
        "expected folder-b in output: {output}"
    );
    assert!(
        output.contains("📁"),
        "expected folder icons in output: {output}"
    );
}

// T-W1-2: RTM FR-28.6
#[test]
fn fr28_6_ac2_list_memories_excludes_hidden_dirs() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_file(&env.cfg, "memory://visible-dir/note", None, "# Test");
    fs::create_dir_all(env.cfg.memory_path.join(".git")).expect("hidden dir");

    let output = memory::list_memories(&env.ctx(), &json!({})).expect("list memories");

    // T-W1-2: RTM FR-28.6 - production list_memories must exclude hidden directories.
    assert!(
        output.contains("visible-dir"),
        "expected visible-dir in output: {output}"
    );
    assert!(
        !output.contains(".git"),
        "hidden .git dir should be excluded: {output}"
    );
}

// T-W1-2: RTM FR-28.6
#[test]
fn fr28_6_ac3_list_memories_folder_filter_returns_only_subset() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_file(&env.cfg, "memory://docs/subfolder/note", None, "# Test");
    seed_file(&env.cfg, "memory://docs/other-sub/note", None, "# Test");
    seed_file(&env.cfg, "memory://notes/archive/note", None, "# Test");

    let output =
        memory::list_memories(&env.ctx(), &json!({ "path": "docs" })).expect("list memories");

    // T-W1-2: RTM FR-28.6 - production list_memories with path filter scopes to that subtree.
    assert!(
        output.contains("subfolder"),
        "expected subfolder in output: {output}"
    );
    assert!(
        output.contains("other-sub"),
        "expected other-sub in output: {output}"
    );
    assert!(
        !output.contains("archive"),
        "notes/archive should not appear: {output}"
    );
    assert!(
        !output.contains("notes"),
        "notes dir should not appear: {output}"
    );
}

// T-W1-2: RTM FR-28.7
#[test]
fn fr28_7_ac1_file_exists_existing_file_returns_true() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_file(&env.cfg, "memory://exists-check", None, "# Test");

    assert!(markdown::file_exists("memory://exists-check", &env.cfg).expect("exists"));
}

// T-W1-2: RTM FR-28.7
#[test]
fn fr28_7_ac2_file_exists_non_existent_file_returns_false() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    assert!(!markdown::file_exists("memory://no-such-file", &env.cfg).expect("exists"));
}

// T-W1-2: RTM FR-28.8
#[test]
fn fr28_8_ac1_delete_file_existing_file_removes_file() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_file(&env.cfg, "memory://delete-me", None, "# Test");
    assert!(markdown::file_exists("memory://delete-me", &env.cfg).expect("precondition"));

    markdown::delete_file("memory://delete-me", &env.cfg).expect("delete file");

    assert!(!markdown::file_exists("memory://delete-me", &env.cfg).expect("postcondition"));
}

// T-W1-2: RTM FR-28.8
#[test]
fn fr28_8_ac2_delete_file_non_existent_file_returns_not_found_error() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let err = markdown::delete_file("memory://never-existed", &env.cfg)
        .expect_err("missing delete target should fail");

    // T-W1-2: RTM FR-28.8 - Assert the error variant (Message), not exact wording.
    // Verify the URI is echoed so the caller can identify which file was missing.
    match &err {
        MaenifoldError::Message(msg) => {
            assert!(
                msg.contains("memory://never-existed"),
                "expected URI echoed in error, got: {msg:?}"
            );
        }
        other => panic!("expected MaenifoldError::Message, got: {other:?}"),
    }
}
