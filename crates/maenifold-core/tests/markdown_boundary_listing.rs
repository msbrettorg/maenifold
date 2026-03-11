#[path = "markdown_boundary/support.rs"]
mod support;

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

    assert!(files.len() >= 3);
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

// T-W1-2: RTM FR-28.5
#[test]
fn fr28_5_ac4_get_memory_files_null_filter_returns_all_files() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_file(&env.cfg, "memory://a", None, "# Test");
    seed_file(&env.cfg, "memory://folder/b", None, "# Test");

    let files = markdown::get_memory_files(&env.cfg, None).expect("all files");

    assert!(files.len() >= 2);
}

// T-W1-2: RTM FR-28.6
#[test]
fn fr28_6_ac1_get_memory_directories_returns_memory_uris() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_file(&env.cfg, "memory://folder-a/note", None, "# Test");
    seed_file(&env.cfg, "memory://folder-b/note", None, "# Test");

    let dirs = get_memory_directories(&env.cfg, None).expect("memory directories");

    assert!(dirs.iter().all(|dir| dir.starts_with("memory://")));
    assert!(dirs.contains(&"memory://folder-a".to_string()));
    assert!(dirs.contains(&"memory://folder-b".to_string()));
}

// T-W1-2: RTM FR-28.6
#[test]
fn fr28_6_ac2_get_memory_directories_excludes_hidden_dirs() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_file(&env.cfg, "memory://visible-dir/note", None, "# Test");
    fs::create_dir_all(env.cfg.memory_path.join(".git")).expect("hidden dir");

    let dirs = get_memory_directories(&env.cfg, None).expect("memory directories");

    assert!(dirs.iter().all(|dir| !dir.contains(".git")));
}

// T-W1-2: RTM FR-28.6
#[test]
fn fr28_6_ac3_get_memory_directories_folder_uri_filter_returns_only_subset() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_file(&env.cfg, "memory://docs/subfolder/note", None, "# Test");
    seed_file(&env.cfg, "memory://docs/other-sub/note", None, "# Test");
    seed_file(&env.cfg, "memory://notes/archive/note", None, "# Test");

    let dirs = get_memory_directories(&env.cfg, Some("memory://docs")).expect("docs dirs");

    assert_eq!(dirs.len(), 2);
    assert!(dirs.iter().all(|dir| dir.starts_with("memory://docs/")));
    assert!(dirs.iter().all(|dir| !dir.starts_with("memory://notes/")));
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

    assert_message_error(err, "Memory file not found");
}
