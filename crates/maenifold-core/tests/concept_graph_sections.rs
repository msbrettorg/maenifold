// T-12.7: RTM NFR-1
#[path = "concept_graph/support.rs"]
mod support;

use maenifold_core::search;
use support::{
    TEST_MUTEX, build_context_json, direct_relations, isolated_env, relation_preview_values,
    sync_graph, write_memory_file,
};

// T-1.6: RTM NFR-1 — ports ConceptGraphTests.cs BuildContextExtractsSectionWithConceptTest.
#[test]
fn build_context_extracts_section_with_concept_test() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    write_memory_file(
        &env,
        "Authentication System Overview",
        "# Authentication System Overview\n\n## Introduction\n\nThis document describes our authentication system architecture.\n\n## Security\n\nThe system uses [[authentication]] via JWT tokens. All requests must include valid [[authentication]] headers.\n\n## Configuration\n\nSee config.yml for settings.",
    );
    write_memory_file(
        &env,
        "JWT Implementation",
        "# JWT Implementation\n\n## Overview\n\nJWT tokens are used for [[authentication]]. This document covers [[JWT]] implementation details.",
    );

    sync_graph(&env);

    let context = build_context_json(&env, "authentication", 1, 20, Some(true));
    let direct = direct_relations(&context);
    assert!(
        !direct.is_empty(),
        "should have direct relations: {context}"
    );

    let preview_values = direct
        .iter()
        .flat_map(relation_preview_values)
        .collect::<Vec<_>>();

    assert!(
        preview_values
            .iter()
            .any(|preview| { preview.contains("## Security") || preview.contains("JWT tokens") }),
        "preview should contain the section where authentication is mentioned: {context}"
    );
    assert!(
        preview_values
            .iter()
            .all(|preview| !preview.starts_with("## Introduction")),
        "preview should not start at the intro section when the concept is in a later section: {context}"
    );
}

// T-1.6: RTM NFR-1 — ports ConceptGraphTests.cs ExtractSectionWithConceptEdgeCasesTest.
#[test]
fn extract_section_with_concept_edge_cases_test() {
    let _guard = TEST_MUTEX.lock();

    let content1 = "# Title\n## Intro\nSome intro.\n## Security\nUsing [[authentication]] here.\n## End\nThe end.";
    let result1 = search::extract_section_with_concept(content1, "authentication", 500);
    assert!(
        result1.contains("## Security"),
        "should extract Security section: {result1}"
    );
    assert!(
        result1.contains("[[authentication]]"),
        "should include the concept mention: {result1}"
    );

    let content2 = "# Title\n## Intro\nMentions [[auth]].\n## Details\nMore [[auth]] info.";
    let result2 = search::extract_section_with_concept(content2, "auth", 500);
    assert!(
        result2.contains("## Intro"),
        "should extract first section with mention: {result2}"
    );

    let content3 = "# Title\nThis uses [[jwt]] tokens.\nMore content here.";
    let result3 = search::extract_section_with_concept(content3, "jwt", 500);
    assert!(
        result3.contains("[[jwt]]"),
        "should extract H1 section: {result3}"
    );

    let content4 = "# Title\n## Section\nNo concept here.";
    let result4 = search::extract_section_with_concept(content4, "nonexistent", 500);
    assert!(
        result4.contains("Title"),
        "should fallback to a preview from the start of the document: {result4}"
    );

    let content5 = "# Title\n## Usage\nPlain authentication without brackets.";
    let result5 = search::extract_section_with_concept(content5, "authentication", 500);
    assert!(
        result5.contains("## Usage"),
        "should match plain text: {result5}"
    );
    assert!(
        result5.contains("authentication"),
        "should include the plain-text mention: {result5}"
    );

    let long_content = format!("# Title\n## Long\nUsing [[test]]. {}", "x".repeat(1000));
    let result6 = search::extract_section_with_concept(&long_content, "test", 100);
    assert!(
        result6.len() <= 200,
        "long sections should be truncated to a compact preview: {}",
        result6.len()
    );

    let result7 = search::extract_section_with_concept("", "test", 500);
    assert!(
        result7.is_empty(),
        "empty content should return an empty preview"
    );

    let result8 = search::extract_section_with_concept("   \n  \n", "test", 500);
    assert!(
        result8.trim().is_empty(),
        "whitespace-only content should not produce a non-empty preview: {result8:?}"
    );
}
