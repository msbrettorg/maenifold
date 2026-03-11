#[path = "markdown_rw_support/mod.rs"]
mod markdown_rw_support;

use maenifold_core::markdown::{append_h2_section, read_markdown};
use markdown_rw_support::{TEST_MUTEX, isolated_env, memory_uri, scalar, write_fixture};

// T-14.2/T-14.3: RTM FR-10 — Focused markdown reader parity regressions.

// T-14.2: RTM FR-10 — Malformed YAML warns and returns empty/null frontmatter.
#[test]
fn read_markdown_malformed_frontmatter_returns_empty_frontmatter() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let relative_path = "reader/malformed-frontmatter.md";
    let uri = memory_uri(relative_path);

    write_fixture(
        &env.cfg,
        relative_path,
        "---\ntitle: [unterminated\n---\n# Body\n\nStill readable.\n",
    );

    let (frontmatter, content, _) = read_markdown(&uri, &env.cfg).expect("read markdown");
    assert!(frontmatter.is_none(), "frontmatter should degrade to None");
    assert_eq!(content, "# Body\n\nStill readable.\n");
}

// T-14.3: RTM FR-10 — CRLF frontmatter delimiters are accepted.
#[test]
fn read_markdown_crlf_frontmatter_parses_correctly() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let relative_path = "reader/crlf-frontmatter.md";
    let uri = memory_uri(relative_path);

    write_fixture(
        &env.cfg,
        relative_path,
        "---\r\ntitle: Windows Note\r\ntags:\r\n  - windows\r\n---\r\n# Heading\r\n\r\nBody\r\n",
    );

    let (frontmatter, content, _) = read_markdown(&uri, &env.cfg).expect("read markdown");
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(
        scalar(&frontmatter, "title").as_deref(),
        Some("Windows Note")
    );
    assert_eq!(content, "# Heading\r\n\r\nBody\r\n");
}

// T-14.3: RTM FR-10 — Closing --- at EOF is still recognized as frontmatter.
#[test]
fn read_markdown_closing_delimiter_at_eof_parses_frontmatter() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let relative_path = "reader/eof-frontmatter.md";
    let uri = memory_uri(relative_path);

    write_fixture(&env.cfg, relative_path, "---\ntitle: EOF Note\n---");

    let (frontmatter, content, _) = read_markdown(&uri, &env.cfg).expect("read markdown");
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(scalar(&frontmatter, "title").as_deref(), Some("EOF Note"));
    assert!(content.is_empty(), "content should be empty at EOF");
}

// T-14.2/T-14.3: RTM FR-10 — Valid YAML regression coverage remains intact.
#[test]
fn read_markdown_valid_frontmatter_still_parses_correctly() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let relative_path = "reader/valid-frontmatter.md";
    let uri = memory_uri(relative_path);

    write_fixture(
        &env.cfg,
        relative_path,
        "---\ntitle: Valid Note\nstatus: active\n---\n# Heading\n\nBody\n",
    );

    let (frontmatter, content, _) = read_markdown(&uri, &env.cfg).expect("read markdown");
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(scalar(&frontmatter, "title").as_deref(), Some("Valid Note"));
    assert_eq!(scalar(&frontmatter, "status").as_deref(), Some("active"));
    assert_eq!(content, "# Heading\n\nBody\n");
}

// T-15.17: RTM NFR-1 — append_h2_section propagates IO errors for existing files.
#[cfg(unix)]
#[test]
fn append_h2_section_propagates_permission_error() {
    use std::os::unix::fs::PermissionsExt;

    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let relative_path = "reader/no-permission.md";
    let uri = memory_uri(relative_path);

    write_fixture(
        &env.cfg,
        relative_path,
        "---\ntitle: Secret\n---\n\nHidden content.\n",
    );

    // Remove read permission so the file exists but cannot be read.
    let path = env.cfg.memory_path.join(relative_path);
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000))
        .expect("set permissions");

    let result = append_h2_section(&uri, "New", "body", &env.cfg);

    // Restore permissions before asserting so cleanup doesn't fail.
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
        .expect("restore permissions");

    assert!(result.is_err(), "permission error must propagate");
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("ermission denied") || err_msg.contains("I/O error"),
        "error should indicate permission problem, got: {err_msg}"
    );
}

// T-15.17: RTM NFR-1 — append_h2_section defaults to empty when file not found.
#[test]
fn append_h2_section_not_found_defaults_to_empty() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = memory_uri("nonexistent-append-target.md");

    // File does not exist — should succeed by creating it.
    append_h2_section(&uri, "First Section", "Hello world", &env.cfg)
        .expect("append to nonexistent file");

    let (_, content, _) = read_markdown(&uri, &env.cfg).expect("read back");
    assert!(
        content.contains("## First Section"),
        "content should contain the new section heading"
    );
    assert!(
        content.contains("Hello world"),
        "content should contain the new section body"
    );
}

// T-15.18: RTM NFR-1 — UTF-8 BOM is stripped, frontmatter parses correctly.
#[test]
fn read_markdown_strips_utf8_bom() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let relative_path = "reader/bom-file.md";
    let uri = memory_uri(relative_path);

    // Write file with leading BOM (U+FEFF) before the frontmatter delimiter.
    write_fixture(
        &env.cfg,
        relative_path,
        "\u{FEFF}---\ntitle: BOM Note\n---\n\nBody after BOM.\n",
    );

    let (frontmatter, content, _) = read_markdown(&uri, &env.cfg).expect("read markdown with BOM");
    let frontmatter = frontmatter.expect("frontmatter should parse after BOM strip");
    assert_eq!(
        scalar(&frontmatter, "title").as_deref(),
        Some("BOM Note"),
        "title must be parsed from BOM-prefixed file"
    );
    assert_eq!(content, "Body after BOM.\n");
}

// T-15.18: RTM NFR-1 — File without BOM is unchanged (regression guard).
#[test]
fn read_markdown_without_bom_unchanged() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let relative_path = "reader/no-bom.md";
    let uri = memory_uri(relative_path);

    write_fixture(
        &env.cfg,
        relative_path,
        "---\ntitle: No BOM\n---\n\nClean content.\n",
    );

    let (frontmatter, content, _) = read_markdown(&uri, &env.cfg).expect("read markdown no BOM");
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(scalar(&frontmatter, "title").as_deref(), Some("No BOM"));
    assert_eq!(content, "Clean content.\n");
}
