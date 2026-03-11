#[path = "markdown_boundary/support.rs"]
mod support;

use support::*;

// T-12.7: RTM NFR-1 - split markdown validation boundary coverage by theme.

// T-W1-2: RTM FR-28.9
#[test]
fn fr28_9_ac1_read_markdown_traversal_uri_throws_invalid_argument() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let err = markdown::read_markdown("memory://../../../etc/passwd", &env.cfg)
        .expect_err("traversal read should fail");

    assert_invalid_argument(err, "path traversal");
}

// T-W1-2: RTM FR-28.9
#[test]
fn fr28_9_ac2_write_markdown_no_memory_prefix_throws_invalid_argument() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let err = markdown::write_markdown("/absolute/path/note", None, "content", &env.cfg)
        .expect_err("missing memory prefix should fail");

    assert_invalid_argument(err, "memory://");
}

// T-W1-2: RTM FR-28.9
#[test]
fn fr28_9_ac3_read_markdown_empty_uri_throws_invalid_argument() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let err = markdown::read_markdown("", &env.cfg).expect_err("empty uri should fail");

    assert_invalid_argument(err, "null or empty");
}

// T-W1-2: RTM FR-28.9
#[test]
fn fr28_9_ac4_file_exists_traversal_uri_throws_invalid_argument() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let err =
        markdown::file_exists("memory://../escape", &env.cfg).expect_err("traversal exists fail");

    assert_invalid_argument(err, "path traversal");
}

// T-W1-2: RTM FR-28.9
#[test]
fn fr28_9_ac5_delete_file_traversal_uri_throws_invalid_argument() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let err =
        markdown::delete_file("memory://../escape", &env.cfg).expect_err("traversal delete fail");

    assert_invalid_argument(err, "path traversal");
}

// T-W1-2: RTM FR-28.9
#[test]
fn fr28_9_ac6_get_memory_files_traversal_folder_uri_throws_invalid_argument() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let err = markdown::get_memory_files(&env.cfg, Some("memory://../escape"))
        .expect_err("traversal folder should fail");

    assert_invalid_argument(err, "path traversal");
}

// T-W1-2: RTM FR-28.9
#[test]
fn fr28_9_ac7_write_markdown_null_uri_throws_invalid_argument() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let err = write_markdown_boundary(None, None, "content", &env.cfg)
        .expect_err("rust boundary adapter should reject null-equivalent uri");

    assert_invalid_argument(err, "null or empty");
}

// T-W1-2: RTM FR-28.9
#[test]
fn fr28_9_ac8_update_markdown_traversal_uri_throws_invalid_argument() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let err = markdown::update_markdown("memory://../escape", None, "content", None, &env.cfg)
        .expect_err("traversal update should fail");

    assert_invalid_argument(err, "path traversal");
}

// T-W1-2: RTM FR-28.9
#[test]
fn fr28_9_ac9_append_h2_section_no_prefix_throws_invalid_argument() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let err = markdown::append_h2_section("just-a-name", "Heading", "Content", &env.cfg)
        .expect_err("missing prefix append should fail");

    assert_invalid_argument(err, "memory://");
}

// T-W1-2: RTM FR-28.9
#[test]
fn fr28_9_ac10_get_memory_directories_traversal_folder_uri_throws_invalid_argument() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let err = get_memory_directories(&env.cfg, Some("memory://../escape"))
        .expect_err("traversal directory filter should fail");

    assert_invalid_argument(err, "path traversal");
}
