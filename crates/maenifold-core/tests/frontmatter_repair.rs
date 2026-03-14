#[path = "frontmatter/support.rs"]
mod support;

use maenifold_core::markdown::{read_markdown, write_markdown};
use maenifold_core::repair;
use serde_json::json;
use serde_yaml::Value as YamlValue;
use std::collections::BTreeMap;
use support::{TEST_MUTEX, isolated_env, markdown_context, yaml_string};

// T-12.6: RTM NFR-1 — Split repair/frontmatter regressions into a focused integration test file.
// T-META-001.6: RTM FR-24.x — Ported frontmatter metadata regression coverage from the C# oracle.

// T-META-001.6: RTM FR-24.4 — repair_concepts uses fs::write directly; it must not alter the
// modified timestamp. Verifies byte-for-byte frontmatter preservation of the modified field.
#[test]
fn fr24_4_ac1_concept_repair_modified_preserved() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let repair_folder = "fr24-repair-tests";
    let file_uri = "memory://fr24-repair-tests/repair-target";

    let mut frontmatter = BTreeMap::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String("Repair Target".to_string()),
    );
    frontmatter.insert("type".to_string(), YamlValue::String("memory".to_string()));
    write_markdown(
        file_uri,
        Some(&frontmatter),
        "# Repair Target\n\n[[tools]] and more [[tools]].",
        &env.cfg,
    )
    .expect("write repair target");

    let (frontmatter, _, _) = read_markdown(file_uri, &env.cfg).expect("read before repair");
    let modified_before =
        yaml_string(&frontmatter.expect("frontmatter"), "modified").expect("modified before");

    let output = repair::repair_concepts(
        &markdown_context(&env.cfg),
        &json!({
            "conceptsToReplace": "tools",
            "canonicalConcept": "tool",
            "folder": repair_folder,
            "dryRun": false,
            "createWikiLinks": false,
            "minSemanticSimilarity": 0.0
        }),
    )
    .expect("repair concepts");
    assert!(!output.starts_with("ERROR:"), "unexpected output: {output}");

    let (frontmatter, _, _) = read_markdown(file_uri, &env.cfg).expect("read after repair");
    let frontmatter = frontmatter.expect("frontmatter after repair");
    let modified_after = yaml_string(&frontmatter, "modified").expect("modified after");
    assert_eq!(
        modified_after, modified_before,
        "repair must not alter modified timestamp (uses fs::write, not write_markdown)"
    );
}

// T-META-001.6: RTM FR-24.4 — ConceptRepair preserves existing frontmatter including sequence types.
#[test]
fn fr24_4_ac2_concept_repair_frontmatter_preserved() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let repair_folder = "fr24-repair-fm-tests";
    let file_uri = "memory://fr24-repair-fm-tests/repair-fm-target";

    let mut frontmatter = BTreeMap::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String("FM Preservation Test".to_string()),
    );
    frontmatter.insert("type".to_string(), YamlValue::String("memory".to_string()));
    frontmatter.insert("status".to_string(), YamlValue::String("saved".to_string()));
    frontmatter.insert(
        "tags".to_string(),
        YamlValue::Sequence(vec![
            YamlValue::String("test".to_string()),
            YamlValue::String("fr24".to_string()),
        ]),
    );
    write_markdown(
        file_uri,
        Some(&frontmatter),
        "# FM Test\n\n[[tools]] are important.",
        &env.cfg,
    )
    .expect("write repair target");

    repair::repair_concepts(
        &markdown_context(&env.cfg),
        &json!({
            "conceptsToReplace": "tools",
            "canonicalConcept": "tool",
            "folder": repair_folder,
            "dryRun": false,
            "createWikiLinks": false,
            "minSemanticSimilarity": 0.0
        }),
    )
    .expect("repair concepts");

    let (frontmatter, content, _) = read_markdown(file_uri, &env.cfg).expect("read after repair");
    let frontmatter = frontmatter.expect("frontmatter after repair");

    // Scalar fields preserved
    assert_eq!(
        yaml_string(&frontmatter, "title").as_deref(),
        Some("FM Preservation Test")
    );
    assert_eq!(yaml_string(&frontmatter, "type").as_deref(), Some("memory"));
    assert_eq!(
        yaml_string(&frontmatter, "status").as_deref(),
        Some("saved")
    );

    // Sequence-type field preserved (tags is a YAML sequence, not a scalar)
    let tags = frontmatter
        .get("tags")
        .expect("tags should be present after repair");
    assert!(
        matches!(tags, YamlValue::Sequence(_)),
        "tags should remain a sequence type: {tags:?}"
    );
    if let YamlValue::Sequence(items) = tags {
        let tag_strings: Vec<&str> = items.iter().filter_map(|v| v.as_str()).collect();
        assert_eq!(
            tag_strings,
            vec!["test", "fr24"],
            "tag values must be preserved after repair"
        );
    }

    // Verify repair actually ran (not a silent no-op)
    assert!(
        content.contains("[[tool]]"),
        "repair should have replaced [[tools]] with [[tool]]: {content}"
    );
    assert!(
        !content.contains("[[tools]]"),
        "original variant should have been replaced: {content}"
    );
}

// NOTE: Test 3 (fr24_4_ac3_concept_repair_replacements_correct) REMOVED per eval verdict MERGE.
// Case-insensitive WikiLink replacement is covered by the stronger
// repair_concepts_matches_wikilinks_case_insensitively test in repair_concepts_batch.rs,
// which exercises 3 case variants and asserts exact replacement count.
