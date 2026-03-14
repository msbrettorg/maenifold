#[path = "markdown_boundary/support.rs"]
mod support;

use support::*;

// T-12.7: RTM NFR-1 - split markdown validation boundary coverage by theme.
// NOTE: URI validation is centralized through `markdown::uri_to_path`, which all
// operations call before any filesystem access. Per-function tests confirm each
// entry point feeds URIs through that gate, catching regressions if a refactor
// bypasses the centralized check.

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

// T-W1-2: RTM FR-28.9 — MERGED ac4 + ac6 + ac10 into parameterized traversal sweep.
// These operations share the same `uri_to_path` validation gate. A single test confirms
// all three reject path traversal without duplicating 60 lines of identical structure.
#[test]
fn fr28_9_traversal_sweep_secondary_operations() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = "memory://../escape";

    // ac4: file_exists
    assert_invalid_argument(
        markdown::file_exists(uri, &env.cfg).expect_err("file_exists should reject traversal"),
        "path traversal",
    );

    // ac6: get_memory_files
    assert_invalid_argument(
        markdown::get_memory_files(&env.cfg, Some(uri))
            .expect_err("get_memory_files should reject traversal"),
        "path traversal",
    );

    // ac10: get_memory_directories (calls get_memory_files internally)
    assert_invalid_argument(
        get_memory_directories(&env.cfg, Some(uri))
            .expect_err("get_memory_directories should reject traversal"),
        "path traversal",
    );
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

// T-W1-2: RTM FR-28.9 — REWRITE ac7: test empty-URI guard on `write_markdown` production API.
// The original tested a test-adapter (`write_markdown_boundary(None, ...)`) which converted
// `None` to `InvalidArgument` — testing test infrastructure, not production code. Rust's
// type system already prevents `None` on `&str` parameters. This rewrite validates the
// production empty-string guard directly.
#[test]
fn fr28_9_ac7_write_markdown_empty_uri_throws_invalid_argument() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let err = markdown::write_markdown("", None, "content", &env.cfg)
        .expect_err("empty uri on write should fail");

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

// T-W1-2: RTM FR-28.9 — Rec 4: URL-encoded path traversal.
// Validates that `%2e%2e` (URL-encoded `..`) does not bypass the traversal guard.
// The URI-level check catches literal `..`; the path-level check catches decoded
// dot sequences via a secondary validation layer (returns Message, not InvalidArgument).
#[test]
fn fr28_9_url_encoded_traversal_is_rejected() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let err = markdown::read_markdown("memory://%2e%2e/escape", &env.cfg)
        .expect_err("URL-encoded traversal should fail");

    assert_message_error(err, "dot sequences");
}
