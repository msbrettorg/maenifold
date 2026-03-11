#[path = "repair_concepts/support.rs"]
mod repair_concepts_support;

use repair_concepts_support::*;
use serde_json::json;

// T-3.2: RTM FR-6 (RTM-038 / TC-6.1)
#[test]
fn repair_concepts_mode_a_replaces_wikilink_variants_on_disk() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(&cfg, "repair/mode-a.md", REPAIR_FIXTURE);

    let output = invoke_repair_concepts(json!({
        "conceptsToReplace": "tools",
        "canonicalConcept": "tool",
        "dryRun": false,
        "minSemanticSimilarity": 0.0
    }));

    let updated = read_memory_fixture(&cfg, "repair/mode-a.md");
    assert!(
        output.contains("✓ Modified: repair/mode-a"),
        "unexpected output: {output}"
    );
    assert!(
        updated.contains("[[tool]]"),
        "canonical concept missing: {updated}"
    );
    assert!(
        !updated.contains("[[tools]]"),
        "variant should be replaced: {updated}"
    );
    assert!(
        updated.contains("[[coding-tool]]") && updated.contains("[[tool system]]"),
        "unrelated concepts should remain unchanged: {updated}"
    );
}

// T-3.2: RTM FR-6 (RTM-040 / TC-6.3)
#[test]
fn repair_concepts_dry_run_returns_summary_without_modifying_files() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let path = "repair/dry-run.md";
    write_memory_fixture(&cfg, path, REPAIR_FIXTURE);
    let original = read_memory_fixture(&cfg, path);

    let output = invoke_repair_concepts(json!({
        "conceptsToReplace": "tools",
        "canonicalConcept": "tool",
        "dryRun": true,
        "minSemanticSimilarity": 0.0
    }));

    let after = read_memory_fixture(&cfg, path);
    assert!(
        output.contains("Would modify: repair/dry-run"),
        "expected dry-run change summary: {output}"
    );
    assert!(
        output.contains("This was a DRY RUN"),
        "missing dry-run marker: {output}"
    );
    assert_eq!(after, original, "dry-run should not modify on-disk content");
}

// T-3.2: RTM FR-6 (RTM-041 / TC-6.4)
#[test]
fn repair_concepts_folder_parameter_restricts_scanning() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(&cfg, "target/inside.md", REPAIR_FIXTURE);
    write_memory_fixture(&cfg, "other/outside.md", REPAIR_FIXTURE);

    let output = invoke_repair_concepts(json!({
        "conceptsToReplace": "tools",
        "canonicalConcept": "tool",
        "folder": "target",
        "dryRun": false,
        "minSemanticSimilarity": 0.0
    }));

    let inside = read_memory_fixture(&cfg, "target/inside.md");
    let outside = read_memory_fixture(&cfg, "other/outside.md");
    assert!(
        output.contains("✓ Modified: target/inside"),
        "expected in-folder modification: {output}"
    );
    assert!(
        !output.contains("other/outside"),
        "out-of-folder file should not be scanned: {output}"
    );
    assert!(
        !inside.contains("[[tools]]"),
        "inside target should be modified: {inside}"
    );
    assert!(
        outside.contains("[[tools]]"),
        "outside target should remain unchanged: {outside}"
    );
}

// T-3.2: RTM FR-6 (RTM-046 / TC-6.9)
#[test]
fn repair_concepts_learn_true_returns_help_documentation() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, _cfg) = isolated_config();

    let output = invoke_repair_concepts(json!({
        "conceptsToReplace": "tools",
        "canonicalConcept": "tool",
        "learn": true
    }));

    assert!(
        output.contains("RepairConcepts"),
        "expected help content to mention tool name: {output}"
    );
    let outside_code_blocks: String = output.split("```").step_by(2).collect();
    assert!(
        !outside_code_blocks.contains("\nScanning "),
        "learn=true should return help instead of live repair output: {output}"
    );
}

// T-3.2: RTM FR-6
#[test]
fn repair_concepts_dry_run_with_invalid_folder_returns_error_output() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, _cfg) = isolated_config();

    let output = invoke_repair_concepts(json!({
        "conceptsToReplace": "tool",
        "canonicalConcept": "build-tool",
        "folder": "nonexistent-folder-xyz",
        "dryRun": true,
        "minSemanticSimilarity": 0.0
    }));

    assert!(
        output.contains("ERROR"),
        "expected invalid folder error output: {output}"
    );
    assert!(
        output.contains("Directory not found"),
        "expected directory-not-found detail: {output}"
    );
}

// T-3.2: RTM FR-6
#[test]
fn repair_concepts_dry_run_with_empty_variants_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, _cfg) = isolated_config();

    let result = try_invoke_repair_concepts(json!({
        "conceptsToReplace": "",
        "canonicalConcept": "generic-tool",
        "dryRun": true,
        "minSemanticSimilarity": 0.0
    }));

    let error = result.expect_err("empty conceptsToReplace should fail");
    assert!(
        error.to_string().contains("No concepts"),
        "expected empty-concepts error message: {error}"
    );
}

// T-3.2: RTM FR-6
#[test]
fn repair_concepts_excludes_git_directories_from_scanning() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(&cfg, "repair/.git/test.md", "[[tool]]");
    write_memory_fixture(&cfg, "repair/visible.md", "No matching concepts here.");

    let output = invoke_repair_concepts(json!({
        "conceptsToReplace": "tool",
        "canonicalConcept": "concept",
        "folder": "repair",
        "dryRun": true,
        "minSemanticSimilarity": 0.0
    }));

    assert!(
        !output.contains(".git"),
        "hidden .git directories should be excluded from repair scanning: {output}"
    );
    assert!(
        output.contains("Files modified: 0"),
        "only hidden matches should yield no visible modifications: {output}"
    );
}

// T-3.2: RTM FR-6
#[test]
fn repair_concepts_semantic_bypass_with_zero_threshold_skips_validation_output() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(&cfg, "repair/semantic-bypass.md", "[[tool]]");

    let output = invoke_repair_concepts(json!({
        "conceptsToReplace": "tool",
        "canonicalConcept": "kitchen-appliance",
        "dryRun": true,
        "minSemanticSimilarity": 0.0
    }));

    assert!(
        !output.contains("SEMANTIC VALIDATION"),
        "threshold 0.0 should bypass semantic validation output: {output}"
    );
    assert!(
        output.contains("Scanning"),
        "repair should still proceed after bypassing semantic validation: {output}"
    );
}
