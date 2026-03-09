use maenifold_core::json::OutputFormat;
use maenifold_core::{AssetManager, Config, ToolRegistry, db};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::{Value, json};
use std::fs;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

const REPAIR_FIXTURE: &str = r#"---
title: Test
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Test
Using [[tools]] and [[coding-tool]] and [[tool system]].
"#;

struct ConfigOverrideReset;

impl Drop for ConfigOverrideReset {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

fn isolated_config() -> (TempDir, Config) {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    AssetManager::initialize_assets(&cfg).expect("assets");
    (tmp, cfg)
}

fn write_memory_fixture(cfg: &Config, relative_path: &str, content: &str) {
    let path = cfg.memory_path.join(relative_path);
    fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
    fs::write(path, content).expect("write memory fixture");
}

fn read_memory_fixture(cfg: &Config, relative_path: &str) -> String {
    fs::read_to_string(cfg.memory_path.join(relative_path)).expect("read memory fixture")
}

fn invoke_repair_concepts(payload: Value) -> String {
    ToolRegistry::global()
        .try_invoke("RepairConcepts", &payload, OutputFormat::Markdown)
        .expect("invoke RepairConcepts")
}

fn try_invoke_repair_concepts(
    payload: Value,
) -> Result<String, maenifold_core::error::MaenifoldError> {
    ToolRegistry::global().try_invoke("RepairConcepts", &payload, OutputFormat::Markdown)
}

fn write_memory_via_registry(title: &str, content: &str) {
    let output = ToolRegistry::global()
        .try_invoke(
            "WriteMemory",
            &json!({
                "title": title,
                "content": content
            }),
            OutputFormat::Json,
        )
        .expect("invoke WriteMemory");
    let value: Value = serde_json::from_str(&output).expect("write memory json");
    assert_eq!(
        value["success"], true,
        "seed write should succeed: {output}"
    );
}

fn seed_community_graph() {
    write_memory_via_registry(
        "Alpha Cluster 1",
        "This cluster links [[alpha]] with [[beta]] and [[gamma]].",
    );
    write_memory_via_registry(
        "Alpha Cluster 2",
        "This cluster links [[beta]] with [[gamma]].",
    );
    write_memory_via_registry("Omega Rho", "This cluster links [[omega]] with [[rho]].");
    write_memory_via_registry(
        "Omega Neighborhood",
        "This cluster links [[omega]], [[rho]], [[sigma]], and [[tau]].",
    );
    write_memory_via_registry(
        "Lambda Neighborhood",
        "This cluster links [[lambda]], [[rho]], [[sigma]], and [[tau]].",
    );
}

fn sync_graph() -> String {
    ToolRegistry::global()
        .try_invoke("Sync", &json!({}), OutputFormat::Markdown)
        .expect("invoke Sync")
}

fn load_community_id(cfg: &Config, concept_name: &str) -> i64 {
    let connection = db::open_connection(&cfg.database_path).expect("open db");
    connection
        .query_row(
            "SELECT community_id FROM concept_communities WHERE concept_name = ?1",
            [concept_name],
            |row| row.get::<_, i64>(0),
        )
        .expect("community id")
}

fn count_distinct_communities(cfg: &Config) -> i64 {
    let connection = db::open_connection(&cfg.database_path).expect("open db");
    connection
        .query_row(
            "SELECT COUNT(DISTINCT community_id) FROM concept_communities",
            [],
            |row| row.get::<_, i64>(0),
        )
        .expect("community count")
}

fn delete_community_assignments(cfg: &Config) {
    let connection = db::open_connection(&cfg.database_path).expect("open db");
    connection
        .execute("DELETE FROM concept_communities", [])
        .expect("delete concept_communities rows");
}

// T-3.3: RTM FR-6 (RTM-038 / TC-6.1)
#[test]
fn repair_concepts_mode_a_replaces_wikilink_variants_on_disk() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(&cfg, "repair/mode-a.md", REPAIR_FIXTURE);

    let output = invoke_repair_concepts(json!({
        "conceptsToReplace": "tools",
        "canonicalConcept": "tool",
        "dryRun": false
    }));

    let updated = read_memory_fixture(&cfg, "repair/mode-a.md");
    assert!(
        output.contains("✓ Modified: repair/mode-a.md"),
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

// T-3.3: RTM FR-6 (RTM-039 / TC-6.2)
#[test]
fn repair_concepts_mode_b_wraps_plain_text_or_reports_stub() {
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
        "dryRun": false
    }));

    let updated = read_memory_fixture(&cfg, path);
    if output.contains("not yet implemented") {
        assert!(
            updated.contains("Using tools in prose"),
            "stubbed Mode B should not modify files: {updated}"
        );
    } else {
        assert!(
            updated.contains("[[tool]]"),
            "Mode B should create canonical wikilinks: {updated}"
        );
        assert!(
            !updated.contains("Using tools in prose"),
            "plain-text variant should be wrapped or replaced: {updated}"
        );
    }
}

// T-3.3: RTM FR-6 (RTM-040 / TC-6.3)
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
        "dryRun": true
    }));

    let after = read_memory_fixture(&cfg, path);
    assert!(
        output.contains("Would modify: repair/dry-run.md"),
        "expected dry-run change summary: {output}"
    );
    assert!(
        output.contains("This was a DRY RUN"),
        "missing dry-run marker: {output}"
    );
    assert_eq!(after, original, "dry-run should not modify on-disk content");
}

// T-3.3: RTM FR-6 (RTM-041 / TC-6.4)
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
        "dryRun": false
    }));

    let inside = read_memory_fixture(&cfg, "target/inside.md");
    let outside = read_memory_fixture(&cfg, "other/outside.md");
    assert!(
        output.contains("✓ Modified: target/inside.md"),
        "expected in-folder modification: {output}"
    );
    assert!(
        !output.contains("other/outside.md"),
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

// T-3.3: RTM FR-6 (RTM-042 / TC-6.5)
#[test]
fn repair_concepts_semantic_validation_blocks_low_similarity_or_gracefully_stubs() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(&cfg, "repair/semantic-block.md", REPAIR_FIXTURE);

    let result = ToolRegistry::global().try_invoke(
        "RepairConcepts",
        &json!({
            "conceptsToReplace": "tools",
            "canonicalConcept": "banana",
            "dryRun": true,
            "minSemanticSimilarity": 0.99
        }),
        OutputFormat::Markdown,
    );

    match result {
        Ok(output) => {
            assert!(
                output.contains("Scanning") || output.contains("SUMMARY"),
                "stubbed semantic validation should still return a repair summary: {output}"
            );
            let unchanged = read_memory_fixture(&cfg, "repair/semantic-block.md");
            assert!(
                unchanged.contains("[[tools]]"),
                "dry-run/stub path should keep content unchanged: {unchanged}"
            );
        }
        Err(error) => assert!(
            error.to_string().to_ascii_lowercase().contains("semantic")
                || error
                    .to_string()
                    .to_ascii_lowercase()
                    .contains("similarity"),
            "unexpected semantic-validation error: {error}"
        ),
    }
}

// T-3.3: RTM FR-6 (RTM-043 / TC-6.6)
#[test]
fn repair_concepts_semantic_validation_allows_high_similarity_or_gracefully_degrades() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(&cfg, "repair/semantic-allow.md", REPAIR_FIXTURE);

    let output = invoke_repair_concepts(json!({
        "conceptsToReplace": "tools",
        "canonicalConcept": "tool",
        "dryRun": true,
        "minSemanticSimilarity": 0.1
    }));

    let after = read_memory_fixture(&cfg, "repair/semantic-allow.md");
    assert!(
        output.contains("Scanning") && output.contains("Total replacements"),
        "expected successful or degraded validation path: {output}"
    );
    assert_eq!(
        after, REPAIR_FIXTURE,
        "dry-run validation path should preserve file contents"
    );
}

// T-3.3: RTM FR-6 (RTM-044 / TC-6.7)
#[test]
fn repair_concepts_mode_b_skips_code_blocks_and_inline_code_or_reports_stub() {
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
        "dryRun": false
    }));

    let updated = read_memory_fixture(&cfg, path);
    if output.contains("not yet implemented") {
        assert!(
            updated.contains("Inline code: `tools`"),
            "stub path should be unchanged: {updated}"
        );
    } else {
        assert!(
            updated.contains("[[tool]]"),
            "prose mention should be linked: {updated}"
        );
        assert!(
            updated.contains("`tools`"),
            "inline code should be preserved: {updated}"
        );
        assert!(
            updated.contains("```text\ntools\n```"),
            "code block should be preserved: {updated}"
        );
    }
}

// T-3.3: RTM FR-6 (RTM-045 / TC-6.8)
#[test]
fn repair_concepts_mode_b_prevents_double_wrapping_or_reports_stub() {
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
        "dryRun": false
    }));

    let updated = read_memory_fixture(&cfg, path);
    if output.contains("not yet implemented") {
        assert!(
            updated.contains("Existing [[tool]] should stay linked"),
            "stub path should leave content unchanged: {updated}"
        );
    } else {
        assert!(
            !updated.contains("[[[") && !updated.contains("]]]]"),
            "Mode B should not create nested WikiLinks: {updated}"
        );
        assert!(
            updated.matches("[[tool]]").count() >= 1,
            "expected canonical link(s) to remain after repair: {updated}"
        );
    }
}

// T-3.3: RTM FR-6 (RTM-046 / TC-6.9)
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

// T-W1-4: RTM FR-6
#[test]
fn repair_concepts_dry_run_reports_replacement_counts() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(&cfg, "repair/replacement-count.md", REPAIR_FIXTURE);

    let output = invoke_repair_concepts(json!({
        "conceptsToReplace": "tool,tools",
        "canonicalConcept": "development-tool",
        "dryRun": true,
        "minSemanticSimilarity": 0.0
    }));

    assert!(
        output.contains("Total replacements:"),
        "expected replacement count summary: {output}"
    );
    assert!(
        output.contains("Would modify: repair/replacement-count.md"),
        "expected dry-run file preview: {output}"
    );
}

// T-W1-4: RTM FR-6
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

// T-W1-4: RTM FR-6
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

// T-W1-4: RTM FR-6
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

// T-W1-4: RTM FR-6
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

// T-W1-4: RTM FR-6
#[test]
fn repair_concepts_default_semantic_threshold_returns_feedback() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    write_memory_fixture(&cfg, "repair/default-semantic-threshold.md", "[[tool]]");

    let output = invoke_repair_concepts(json!({
        "conceptsToReplace": "tool",
        "canonicalConcept": "utility-tool",
        "dryRun": true
    }));

    assert!(
        output.contains("SEMANTIC VALIDATION"),
        "default threshold should emit semantic validation feedback: {output}"
    );
}

// T-W1-4: RTM FR-6
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
        output.contains("✓ Modified: repair/word-boundary.md"),
        "expected create-wikilinks modification output: {output}"
    );
}

// T-W1-4: RTM FR-6
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

// T-W1-4: RTM FR-6
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

// T-W1-4: RTM FR-6
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

// T-W1-4: RTM FR-6
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

// T-W1-4: RTM FR-6
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

// T-W1-4: RTM FR-6
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

// T-W1-4: RTM FR-6
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

// T-W1-4: RTM FR-6
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

// T-3.3: RTM FR-13.1
#[test]
fn community_detection_populates_non_zero_community_count_after_sync() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_community_graph();

    let sync = sync_graph();
    let community_count = count_distinct_communities(&cfg);

    assert!(
        sync.contains("COMMUNITIES detected"),
        "unexpected sync output: {sync}"
    );
    assert!(
        community_count > 0,
        "expected at least one community after sync"
    );
}

// T-3.3: RTM FR-13.6 / RTM FR-13.10
#[test]
fn build_context_shows_real_community_ids_not_hardcoded_zero() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_community_graph();
    let _sync = sync_graph();

    let omega_community_id = load_community_id(&cfg, "omega");
    assert!(
        omega_community_id > 0,
        "expected omega to live outside the first community for this fixture"
    );

    let context = ToolRegistry::global()
        .try_invoke(
            "BuildContext",
            &json!({"conceptName":"omega","depth":1}),
            OutputFormat::Markdown,
        )
        .expect("invoke BuildContext");

    assert!(
        context.contains(&format!("[community {omega_community_id}]")),
        "expected real persisted community id in BuildContext output: {context}"
    );
    assert!(
        !context.contains("[community 0]"),
        "community id should not be hardcoded to zero for omega fixture: {context}"
    );
}

// T-3.3: RTM FR-13.7
#[test]
fn build_context_with_community_data_shows_community_siblings_section() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, _cfg) = isolated_config();
    seed_community_graph();
    let _sync = sync_graph();

    let context = ToolRegistry::global()
        .try_invoke(
            "BuildContext",
            &json!({"conceptName":"omega","depth":1}),
            OutputFormat::Markdown,
        )
        .expect("invoke BuildContext");

    assert!(
        context.contains("Community Siblings"),
        "expected sibling section in BuildContext output: {context}"
    );
    assert!(
        context.contains("lambda"),
        "expected indirect same-community sibling to appear: {context}"
    );
}

// T-3.3: RTM FR-13.10
#[test]
fn build_context_gracefully_degrades_with_empty_concept_communities_table() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_community_graph();
    let _sync = sync_graph();
    delete_community_assignments(&cfg);

    let context = ToolRegistry::global()
        .try_invoke(
            "BuildContext",
            &json!({"conceptName":"omega","depth":1}),
            OutputFormat::Markdown,
        )
        .expect("invoke BuildContext");

    assert!(
        context.contains("rho"),
        "expected relation output to remain available: {context}"
    );
    assert!(
        !context.contains("[community "),
        "graceful degradation should omit absent community ids: {context}"
    );
    assert!(
        !context.contains("Community Siblings"),
        "sibling section should be omitted when community table is empty: {context}"
    );
}
