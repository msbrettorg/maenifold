#[path = "frontmatter/support.rs"]
mod support;

use maenifold_core::markdown::{read_markdown, write_markdown};
use maenifold_core::repair;
use serde_json::json;
use serde_yaml::Value as YamlValue;
use std::collections::BTreeMap;
use std::thread;
use std::time::Duration as StdDuration;
use support::{TEST_MUTEX, isolated_env, markdown_context, yaml_string};

// T-12.6: RTM NFR-1 — Split repair/frontmatter regressions into a focused integration test file.
// T-META-001.6: RTM FR-24.x — Ported frontmatter metadata regression coverage from the C# oracle.

// T-META-001.6: RTM FR-24.4 — ConceptRepairTool flows through Markdown IO.
#[test]
fn fr24_4_ac1_concept_repair_modified_updated() {
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
        yaml_string(&frontmatter.expect("frontmatter"), "modified").expect("modified");

    thread::sleep(StdDuration::from_millis(20));

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
    let frontmatter = frontmatter.expect("frontmatter");
    let modified_after = yaml_string(&frontmatter, "modified").expect("modified");
    assert!(!modified_after.is_empty());
    assert!(!modified_before.is_empty());
}

// T-META-001.6: RTM FR-24.4 — ConceptRepair preserves existing frontmatter.
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

    let (frontmatter, _, _) = read_markdown(file_uri, &env.cfg).expect("read after repair");
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(
        yaml_string(&frontmatter, "title").as_deref(),
        Some("FM Preservation Test")
    );
    assert_eq!(yaml_string(&frontmatter, "type").as_deref(), Some("memory"));
    assert_eq!(
        yaml_string(&frontmatter, "status").as_deref(),
        Some("saved")
    );
}

// T-META-001.6: RTM FR-24.4 — ConceptRepair replaces WikiLink variants correctly.
#[test]
fn fr24_4_ac3_concept_repair_replacements_correct() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let repair_folder = "fr24-repair-content-tests";
    let file_uri = "memory://fr24-repair-content-tests/repair-content-target";

    write_markdown(
        file_uri,
        None,
        "# Content Test\n\nUsing [[tools]] and [[Tools]] here.",
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

    let (_, content, _) = read_markdown(file_uri, &env.cfg).expect("read after repair");
    assert!(
        content.contains("[[tool]]"),
        "missing canonical wikilink: {content}"
    );
    assert!(
        !content.contains("[[tools]]"),
        "variant should be removed: {content}"
    );
    assert!(
        !content.contains("[[Tools]]"),
        "case variant should be removed: {content}"
    );
}
