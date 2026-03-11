#[path = "repair_concepts/support.rs"]
mod repair_concepts_support;

use repair_concepts_support::*;
use serde_json::json;

// T-3.2: RTM FR-6
#[test]
fn repair_concepts_can_replace_multiple_variants_in_one_operation() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let path = "repair/multi-replace.md";
    write_memory_fixture(
        &cfg,
        path,
        "# Test\n[[tool]] [[tools]] [[utility]] work together.\nThe [[tool]] is essential.\n",
    );

    let output = invoke_repair_concepts(json!({
        "conceptsToReplace": "tool,tools,utility",
        "canonicalConcept": "generic-tool",
        "dryRun": false,
        "minSemanticSimilarity": 0.0
    }));

    let updated = read_memory_fixture(&cfg, path);
    assert!(
        !updated.contains("[[tool]]")
            && !updated.contains("[[tools]]")
            && !updated.contains("[[utility]]"),
        "all specified variants should be replaced: {updated}"
    );
    assert!(
        updated.matches("[[generic-tool]]").count() == 4,
        "expected four canonical replacements in output file: {updated}"
    );
    assert!(
        output.contains("Total replacements: 4"),
        "expected total replacement count of 4: {output}"
    );
}

// T-3.2: RTM FR-6
#[test]
fn repair_concepts_modifies_multiple_files_in_a_single_batch() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(
        &cfg,
        "repair/batch-file1.md",
        "# File 1\n\n[[agent]] and more content\n",
    );
    write_memory_fixture(
        &cfg,
        "repair/batch-file2.md",
        "# File 2\n\n[[agent]] in different file\n",
    );
    write_memory_fixture(
        &cfg,
        "repair/batch-file3.md",
        "# File 3\n\nNo matching concepts here\n",
    );

    let output = invoke_repair_concepts(json!({
        "conceptsToReplace": "agent",
        "canonicalConcept": "ai-agent",
        "folder": "repair",
        "dryRun": false,
        "minSemanticSimilarity": 0.0
    }));

    assert!(
        read_memory_fixture(&cfg, "repair/batch-file1.md").contains("[[ai-agent]]"),
        "first matching file should be modified"
    );
    assert!(
        read_memory_fixture(&cfg, "repair/batch-file2.md").contains("[[ai-agent]]"),
        "second matching file should be modified"
    );
    assert!(
        read_memory_fixture(&cfg, "repair/batch-file3.md").contains("No matching concepts here"),
        "non-matching file should remain unchanged"
    );
    assert!(
        output.contains("Files modified: 2"),
        "summary should report two modified files: {output}"
    );
}

// T-3.2: RTM FR-6
#[test]
fn repair_concepts_handles_empty_files_gracefully() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let path = "repair/empty-file.md";
    write_memory_fixture(&cfg, path, "");

    let output = invoke_repair_concepts(json!({
        "conceptsToReplace": "tool",
        "canonicalConcept": "testing-tool",
        "dryRun": false,
        "minSemanticSimilarity": 0.0
    }));

    assert_eq!(
        read_memory_fixture(&cfg, path),
        "",
        "empty file should remain empty"
    );
    assert!(
        output.contains("Files modified: 0"),
        "empty file should not produce modifications: {output}"
    );
}

// T-3.2: RTM FR-6
#[test]
fn repair_concepts_handles_whitespace_only_files_gracefully() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let path = "repair/whitespace-file.md";
    write_memory_fixture(&cfg, path, "   \n\n   \t\n");

    let output = invoke_repair_concepts(json!({
        "conceptsToReplace": "tool",
        "canonicalConcept": "automation-tool",
        "dryRun": false,
        "minSemanticSimilarity": 0.0
    }));

    assert!(
        read_memory_fixture(&cfg, path).trim().is_empty(),
        "whitespace-only file should remain whitespace-only"
    );
    assert!(
        output.contains("Files modified: 0"),
        "whitespace-only file should not be modified: {output}"
    );
}

// T-3.2: RTM FR-6
#[test]
fn repair_concepts_replaces_variants_with_special_characters() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let path = "repair/special-chars.md";
    write_memory_fixture(&cfg, path, "[[c#]] [[c++]] [[.net]]");

    let output = invoke_repair_concepts(json!({
        "conceptsToReplace": "c#,c++,.net",
        "canonicalConcept": "dotnet-ecosystem",
        "dryRun": false,
        "minSemanticSimilarity": 0.0
    }));

    let updated = read_memory_fixture(&cfg, path);
    assert!(
        updated.matches("[[dotnet-ecosystem]]").count() == 3,
        "special-character concepts should all be replaced: {updated}"
    );
    assert!(
        output.contains("Total replacements: 3"),
        "expected three replacements for special-character concepts: {output}"
    );
}

// T-3.2: RTM FR-6
#[test]
fn repair_concepts_matches_wikilinks_case_insensitively() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let path = "repair/case-sensitivity.md";
    write_memory_fixture(&cfg, path, "[[Tool]] [[TOOL]] [[tool]]");

    let output = invoke_repair_concepts(json!({
        "conceptsToReplace": "tool",
        "canonicalConcept": "unified-tool",
        "dryRun": false,
        "minSemanticSimilarity": 0.0
    }));

    let updated = read_memory_fixture(&cfg, path);
    assert!(
        !updated.contains("[[Tool]]")
            && !updated.contains("[[TOOL]]")
            && !updated.contains("[[tool]]"),
        "case-insensitive matching should replace all case variants: {updated}"
    );
    assert!(
        updated.matches("[[unified-tool]]").count() == 3,
        "all case variants should normalize to the canonical concept: {updated}"
    );
    assert!(
        output.contains("Total replacements: 3"),
        "summary should report three case-insensitive replacements: {output}"
    );
}

// T-3.2: RTM FR-6
#[test]
fn repair_concepts_scans_nested_folder_depths() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(&cfg, "repair/level1/file1.md", "[[database-schema]]");
    write_memory_fixture(&cfg, "repair/level1/level2/file2.md", "[[database-schema]]");
    write_memory_fixture(
        &cfg,
        "repair/level1/level2/level3/file3.md",
        "[[database-schema]]",
    );

    let output = invoke_repair_concepts(json!({
        "conceptsToReplace": "database-schema",
        "canonicalConcept": "data-model",
        "folder": "repair",
        "dryRun": false,
        "minSemanticSimilarity": 0.0
    }));

    assert!(
        read_memory_fixture(&cfg, "repair/level1/file1.md").contains("[[data-model]]"),
        "level 1 file should be modified"
    );
    assert!(
        read_memory_fixture(&cfg, "repair/level1/level2/file2.md").contains("[[data-model]]"),
        "level 2 file should be modified"
    );
    assert!(
        read_memory_fixture(&cfg, "repair/level1/level2/level3/file3.md")
            .contains("[[data-model]]"),
        "level 3 file should be modified"
    );
    assert!(
        output.contains("Files modified: 3"),
        "nested folder scan should report all three modified files: {output}"
    );
}
