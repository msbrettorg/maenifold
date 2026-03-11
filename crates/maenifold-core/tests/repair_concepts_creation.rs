#[path = "repair_concepts/support.rs"]
mod repair_concepts_support;

use repair_concepts_support::*;
use serde_json::json;

// T-10.10: RTM FR-17 / RTM-094
#[test]
fn repair_concepts_mode_b_wraps_plain_text_mentions_with_canonical_wikilinks() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let path = "repair/mode-b-wrap.md";
    write_memory_fixture(
        &cfg,
        path,
        r#"---
title: Mode B Wrap
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Test
Using tools in prose should become a canonical wikilink.
"#,
    );

    let output = invoke_repair_concepts(json!({
        "conceptsToReplace": "tools",
        "canonicalConcept": "tool",
        "createWikiLinks": true,
        "dryRun": false,
        "minSemanticSimilarity": 0.0
    }));

    let updated = read_memory_fixture(&cfg, path);
    assert!(
        output.contains("✓ Modified: repair/mode-b-wrap"),
        "Mode B should report the modified file: {output}"
    );
    assert!(
        output.contains("Total replacements: 1"),
        "Mode B should report one replacement for the plain-text mention: {output}"
    );
    assert!(
        updated.contains("Using [[tool]] in prose should become a canonical wikilink."),
        "Mode B should wrap the plain-text mention with the canonical wikilink: {updated}"
    );
    assert!(
        !updated.contains("Using tools in prose should become a canonical wikilink."),
        "the original plain-text variant should no longer remain in prose: {updated}"
    );
}

// T-10.10: RTM FR-17 / RTM-096
#[test]
fn repair_concepts_mode_b_skips_code_blocks_and_inline_code() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let path = "repair/mode-b-code.md";
    write_memory_fixture(
        &cfg,
        path,
        r#"---
title: Mode B Code
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Test
tools should be wrapped in prose.

Inline code: `tools`

```text
tools
```
"#,
    );

    let output = invoke_repair_concepts(json!({
        "conceptsToReplace": "tools",
        "canonicalConcept": "tool",
        "createWikiLinks": true,
        "dryRun": false,
        "minSemanticSimilarity": 0.0
    }));

    let updated = read_memory_fixture(&cfg, path);
    assert!(
        output.contains("✓ Modified: repair/mode-b-code"),
        "Mode B should report the modified file: {output}"
    );
    assert!(
        output.contains("Total replacements: 1"),
        "only the prose mention should be replaced: {output}"
    );
    assert!(
        updated.contains("[[tool]] should be wrapped in prose."),
        "prose mention should be wrapped with the canonical wikilink: {updated}"
    );
    assert!(
        updated.contains("Inline code: `tools`"),
        "inline code should be preserved without wikilink creation: {updated}"
    );
    assert!(
        updated.contains("```text\ntools\n```"),
        "fenced code blocks should be preserved without wikilink creation: {updated}"
    );
}

// T-10.10: RTM FR-17 / RTM-097
#[test]
fn repair_concepts_mode_b_does_not_double_wrap_existing_wikilinks() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let path = "repair/mode-b-double-wrap.md";
    write_memory_fixture(
        &cfg,
        path,
        r#"---
title: Mode B Double Wrap
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Test
Existing [[tool]] should stay linked while tools in prose may be added.
"#,
    );

    let output = invoke_repair_concepts(json!({
        "conceptsToReplace": "tools",
        "canonicalConcept": "tool",
        "createWikiLinks": true,
        "dryRun": false,
        "minSemanticSimilarity": 0.0
    }));

    let updated = read_memory_fixture(&cfg, path);
    assert!(
        output.contains("✓ Modified: repair/mode-b-double-wrap"),
        "Mode B should report the modified file: {output}"
    );
    assert!(
        output.contains("Total replacements: 1"),
        "only the plain-text mention should be linked: {output}"
    );
    assert!(
        !updated.contains("[[[") && !updated.contains("]]]]"),
        "Mode B should not create nested WikiLinks: {updated}"
    );
    assert!(
        updated.matches("[[tool]]").count() == 2,
        "the existing wikilink should remain single-wrapped and the prose mention should add exactly one new canonical link: {updated}"
    );
    assert!(
        updated
            .contains("Existing [[tool]] should stay linked while [[tool]] in prose may be added."),
        "existing wikilinks should remain intact while new plain-text mentions are linked: {updated}"
    );
}

// T-3.2: RTM FR-6
#[test]
fn repair_concepts_create_wikilinks_mode_respects_word_boundaries() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let path = "repair/word-boundary.md";
    write_memory_fixture(
        &cfg,
        path,
        "# Test\nThe tool is used in toolbox management.\nThis is a toolkit.\nNo: the tool (parentheses)\n",
    );

    let output = invoke_repair_concepts(json!({
        "conceptsToReplace": "tool",
        "canonicalConcept": "software-tool",
        "createWikiLinks": true,
        "dryRun": false,
        "minSemanticSimilarity": 0.0
    }));

    let updated = read_memory_fixture(&cfg, path);
    assert!(
        updated.matches("[[software-tool]]").count() == 2,
        "standalone word matches should be wrapped exactly twice: {updated}"
    );
    assert!(
        updated.contains("toolbox") && updated.contains("toolkit"),
        "embedded substrings should remain unchanged: {updated}"
    );
    assert!(
        output.contains("✓ Modified: repair/word-boundary"),
        "expected create-wikilinks modification output: {output}"
    );
}

// T-3.2: RTM FR-6
#[test]
fn repair_concepts_create_wikilinks_preserves_existing_wikilinks() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let path = "repair/existing-wikilinks.md";
    let original = "# Test\nAlready [[existing-concept]] with tool text.\n[[existing-concept]] is already a link.\ntool should be converted but [[existing-concept]] should stay.\n";
    write_memory_fixture(&cfg, path, original);

    let output = invoke_repair_concepts(json!({
        "conceptsToReplace": "tool",
        "canonicalConcept": "build-tool",
        "createWikiLinks": true,
        "dryRun": false,
        "minSemanticSimilarity": 0.0
    }));

    let updated = read_memory_fixture(&cfg, path);
    assert_eq!(
        updated.matches("[[existing-concept]]").count(),
        original.matches("[[existing-concept]]").count(),
        "existing wikilinks should be preserved without modification: {updated}"
    );
    assert!(
        updated.contains("[[build-tool]]"),
        "plain-text tool mentions should become build-tool wikilinks: {updated}"
    );
    assert!(
        output.contains("build-tool"),
        "expected canonical concept mention in output: {output}"
    );
}
