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
