#[path = "markdown_rw_support/mod.rs"]
mod markdown_rw_support;

use maenifold_core::markdown::{extract_wikilinks_with_sections, read_markdown};
use markdown_rw_support::{
    TEST_MUTEX, count_all_concept_occurrences, file_concepts, isolated_env, memory_uri,
    mention_count, scalar, sync_one, write_fixture,
};
use std::collections::{BTreeMap, BTreeSet};

// T-12.7.1: RTM NFR-1 — Split markdown sync regressions into focused sync coverage.

#[test]
fn sync_basic_file_frontmatter_and_content_match_expected_processing() {
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
    let concepts_with_sections = extract_wikilinks_with_sections(&content);
    let concepts = concepts_with_sections
        .iter()
        .map(|(concept, _)| concept.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let concept_counts = count_all_concept_occurrences(&content, &concepts);
    let output = sync_one(&env.cfg, relative_path);

    assert_eq!(scalar(&frontmatter, "title").as_deref(), Some("Test Note"));
    assert!(content.contains("[[alpha]]"));
    assert!(!checksum.is_empty());
    assert_eq!(
        concepts,
        vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()]
    );
    assert_eq!(
        concept_counts,
        BTreeMap::from([
            ("alpha".to_string(), 1usize),
            ("beta".to_string(), 1usize),
            ("gamma".to_string(), 1usize),
        ])
    );
    assert!(output.contains("Synced memory://sync/basic"));
    assert_eq!(
        file_concepts(&env.cfg, &uri),
        concepts.iter().cloned().collect::<BTreeSet<_>>()
    );
    assert_eq!(mention_count(&env.cfg, &uri), 3);
}

#[test]
fn sync_wikilinks_with_sections_section_indices_match_expected_extraction() {
    let content = r#"Preamble mentions [[preamble-concept]].

## First Section

Content with [[alpha]] and [[beta]].

## Second Section

Content with [[gamma]].
Also [[alpha]] appears again in section two.
"#;

    let concepts_with_sections = extract_wikilinks_with_sections(content);

    assert!(concepts_with_sections.contains(&("preamble-concept".to_string(), 0)));
    assert!(concepts_with_sections.contains(&("alpha".to_string(), 1)));
    assert!(concepts_with_sections.contains(&("beta".to_string(), 1)));
    assert!(concepts_with_sections.contains(&("gamma".to_string(), 2)));
    assert!(concepts_with_sections.contains(&("alpha".to_string(), 2)));
}

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
}

#[test]
fn sync_no_frontmatter_content_only_file_is_processed() {
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
    let output = sync_one(&env.cfg, relative_path);

    assert!(frontmatter.is_none());
    assert!(content.contains("[[some-concept]]"));
    assert!(!checksum.is_empty());
    assert!(output.contains("2 concept mentions updated"));
    assert_eq!(
        file_concepts(&env.cfg, &uri),
        BTreeSet::from(["another-concept".to_string(), "some-concept".to_string()])
    );
}

#[test]
fn sync_empty_file_is_handled_gracefully() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let relative_path = "sync/empty.md";
    let uri = memory_uri(relative_path);
    write_fixture(&env.cfg, relative_path, "");

    let (frontmatter, content, checksum) = read_markdown(&uri, &env.cfg).expect("read");
    let output = sync_one(&env.cfg, relative_path);

    assert!(frontmatter.is_none());
    assert_eq!(content, "");
    assert!(!checksum.is_empty());
    assert!(output.contains("0 concept mentions updated"));
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
}
