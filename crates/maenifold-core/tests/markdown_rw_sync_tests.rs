#[path = "markdown_rw_support/mod.rs"]
mod markdown_rw_support;

use maenifold_core::markdown::read_markdown;
use markdown_rw_support::{
    TEST_MUTEX, count_all_concept_occurrences, file_concepts, isolated_env, memory_uri,
    mention_count, scalar, sync_one, write_fixture,
};
use std::collections::BTreeSet;

// T-12.7.1: RTM NFR-1 — Split markdown sync regressions into focused sync coverage.

#[test]
fn sync_basic_file_db_state_matches_expected_after_sync() {
    // T-12.7.1: RTM NFR-1 — REWRITE: stripped redundant extract_wikilinks_with_sections
    // and count_all_concept_occurrences assertions (covered by proximity_weight_sections.rs
    // and T3 respectively). Focus is read_markdown + sync_one + DB state verification.
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let relative_path = "sync/basic.md";
    let uri = memory_uri(relative_path);
    write_fixture(
        &env.cfg,
        relative_path,
        r#"---
title: Test Note
tags:
  - alpha
  - beta
---

This note mentions [[alpha]] and [[beta]].
Also references [[gamma]] for good measure.
"#,
    );

    let (frontmatter, content, checksum) = read_markdown(&uri, &env.cfg).expect("read");
    let frontmatter = frontmatter.expect("frontmatter");
    let output = sync_one(&env.cfg, relative_path);

    assert_eq!(scalar(&frontmatter, "title").as_deref(), Some("Test Note"));
    assert!(content.contains("[[alpha]]"));
    assert!(!checksum.is_empty());
    assert!(output.contains("Synced memory://sync/basic"));
    assert_eq!(
        file_concepts(&env.cfg, &uri),
        BTreeSet::from([
            "alpha".to_string(),
            "beta".to_string(),
            "gamma".to_string(),
        ])
    );
    assert_eq!(mention_count(&env.cfg, &uri), 3);
}

// T-12.7.1: T2 REMOVED — extract_wikilinks_with_sections is exhaustively tested
// in proximity_weight_sections.rs with stronger exact-match assertions. Zero unique coverage.

#[test]
fn sync_concept_counts_match_expected_occurrences() {
    let content = r#"[[alpha]] appears first.

Then [[beta]] appears.

Later [[alpha]] appears again, and [[alpha]] a third time.

[[beta]] also appears a second time.
"#;
    let concepts = vec!["alpha".to_string(), "beta".to_string()];
    let concept_counts = count_all_concept_occurrences(content, &concepts);

    assert_eq!(concept_counts["alpha"], 3);
    assert_eq!(concept_counts["beta"], 2);
}

#[test]
fn sync_code_block_exclusion_ignores_wikilinks_in_fenced_code_blocks() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let relative_path = "sync/codeblocks.md";
    let uri = memory_uri(relative_path);
    write_fixture(
        &env.cfg,
        relative_path,
        r#"This paragraph mentions [[real-concept]].

```markdown
This code block contains [[fake-concept]] that must be ignored.
```

Another paragraph with [[another-real-concept]].
"#,
    );

    let output = sync_one(&env.cfg, relative_path);
    let concepts = file_concepts(&env.cfg, &uri);

    assert!(output.contains("Synced memory://sync/codeblocks"));
    assert!(concepts.contains("real-concept"));
    assert!(concepts.contains("another-real-concept"));
    assert!(!concepts.contains("fake-concept"));
    // T-12.7.1: P4 gap closure — verify exclusion doesn't silently inflate mention count
    assert_eq!(mention_count(&env.cfg, &uri), 2);
}

#[test]
fn sync_no_frontmatter_content_only_file_is_processed() {
    // T-12.7.1: RTM NFR-1 — REWRITE: replaced brittle output.contains("2 concept mentions
    // updated") with structural mention_count DB assertion.
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let relative_path = "sync/no-frontmatter.md";
    let uri = memory_uri(relative_path);
    write_fixture(
        &env.cfg,
        relative_path,
        "Just a plain note with [[some-concept]] and [[another-concept]].\n\nNo frontmatter block at all.\n",
    );

    let (frontmatter, content, checksum) = read_markdown(&uri, &env.cfg).expect("read");
    let _output = sync_one(&env.cfg, relative_path);

    assert!(frontmatter.is_none());
    assert!(content.contains("[[some-concept]]"));
    assert!(!checksum.is_empty());
    assert_eq!(mention_count(&env.cfg, &uri), 2);
    assert_eq!(
        file_concepts(&env.cfg, &uri),
        BTreeSet::from(["another-concept".to_string(), "some-concept".to_string()])
    );
}

#[test]
fn sync_empty_file_is_handled_gracefully() {
    // T-12.7.1: RTM NFR-1 — REWRITE: removed trivially-true !checksum.is_empty() (SHA of
    // empty is always non-empty) and brittle output string assertion. Replaced with
    // structural mention_count DB check.
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let relative_path = "sync/empty.md";
    let uri = memory_uri(relative_path);
    write_fixture(&env.cfg, relative_path, "");

    let (frontmatter, content, _checksum) = read_markdown(&uri, &env.cfg).expect("read");
    let _output = sync_one(&env.cfg, relative_path);

    assert!(frontmatter.is_none());
    assert_eq!(content, "");
    assert_eq!(mention_count(&env.cfg, &uri), 0);
    assert!(file_concepts(&env.cfg, &uri).is_empty());
}

#[test]
fn sync_inline_code_exclusion_ignores_wikilinks_in_backticks() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let relative_path = "sync/inline-code.md";
    let uri = memory_uri(relative_path);
    write_fixture(
        &env.cfg,
        relative_path,
        r#"This mentions [[real-concept]] normally.

But `[[fake-inline-concept]]` is inside backticks and must be ignored.

And [[another-real]] is also valid.
"#,
    );

    let output = sync_one(&env.cfg, relative_path);
    let concepts = file_concepts(&env.cfg, &uri);

    assert!(output.contains("Synced memory://sync/inline-code"));
    assert!(concepts.contains("real-concept"));
    assert!(concepts.contains("another-real"));
    assert!(!concepts.contains("fake-inline-concept"));
    // T-12.7.1: P4 gap closure — verify exclusion doesn't silently inflate mention count
    assert_eq!(mention_count(&env.cfg, &uri), 2);
}

#[test]
fn sync_extract_excludes_wikilinks_in_all_code_contexts() {
    // T-12.7: RTM NFR-1 — MERGE from vector_sync_boundary.rs: combined fenced + inline code
    // exclusion through the Sync+Extract pipeline. Tightened positive assertion to structural
    // concept-set check instead of substring matching. Original name was
    // markdown_reader_sync_should_match_single_pass_extraction_contract.
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let relative_path = "sync/code-exclusion-combined.md";
    let uri = memory_uri(relative_path);
    write_fixture(
        &env.cfg,
        relative_path,
        "Alpha [[authentication]] outside code.\n\n```rust\n[[ignored-in-code]]\n```\n\nInline `[[ignored-inline]]` should also be excluded.\n",
    );

    let output = sync_one(&env.cfg, relative_path);
    let concepts = file_concepts(&env.cfg, &uri);

    assert!(output.contains("Synced memory://sync/code-exclusion-combined"));
    assert_eq!(
        concepts,
        BTreeSet::from(["authentication".to_string()]),
        "only non-code wikilinks should appear as concepts: {concepts:?}"
    );
    assert!(!concepts.contains("ignored-in-code"));
    assert!(!concepts.contains("ignored-inline"));
    assert_eq!(mention_count(&env.cfg, &uri), 1);
}
