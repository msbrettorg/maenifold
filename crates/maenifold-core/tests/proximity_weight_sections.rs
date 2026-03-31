// T-12.0: RTM NFR-1
#[path = "proximity_weight/support.rs"]
mod support;

use maenifold_core::markdown::extract_wikilinks_with_sections;
use std::collections::BTreeMap;
use support::{
    TEST_MUTEX, TestEnv, assert_approx_eq, invoke_sync, open_db, relation_source_files,
    relation_weight_for_uri, write_memory_file,
};

// T-4.3: RTM FR-23.1
#[test]
fn extract_wikilinks_with_sections_tags_multi_section_documents() {
    let _guard = TEST_MUTEX.lock();

    let content = "# Document Title\n\n## Section One\n\nDiscusses [[alpha]] and [[beta]].\n\n## Section Two\n\nIntroduces [[gamma]] and [[delta]].\n";
    let concepts = extract_wikilinks_with_sections(content);
    let lookup = concepts.into_iter().collect::<BTreeMap<_, _>>();

    assert_eq!(lookup.get("alpha"), Some(&1));
    assert_eq!(lookup.get("beta"), Some(&1));
    assert_eq!(lookup.get("gamma"), Some(&2));
    assert_eq!(lookup.get("delta"), Some(&2));
}

// T-4.3: RTM FR-23.1
#[test]
fn extract_wikilinks_with_sections_uses_section_zero_for_flat_docs() {
    let _guard = TEST_MUTEX.lock();

    let content = "# Just a Title\n\nThis note references [[concepta]], [[conceptb]], and [[conceptc]].\nNo H2 headings anywhere.\n";
    let concepts = extract_wikilinks_with_sections(content);

    assert!(!concepts.is_empty(), "flat doc should extract concepts");
    assert!(
        concepts
            .iter()
            .all(|(_, section_index)| *section_index == 0),
        "every flat-doc concept must remain in section zero: {concepts:?}"
    );
}

// T-4.3: RTM FR-23.1
#[test]
fn extract_wikilinks_with_sections_keeps_preamble_in_section_zero() {
    let _guard = TEST_MUTEX.lock();

    let content = "# Document\n\nThis preamble mentions [[preamble-concept]] before any heading.\n\n## First Section\n\nThis section has [[section-concept]].\n";
    let concepts = extract_wikilinks_with_sections(content);
    let lookup = concepts.into_iter().collect::<BTreeMap<_, _>>();

    assert_eq!(lookup.get("preamble-concept"), Some(&0));
    assert_eq!(lookup.get("section-concept"), Some(&1));
}

// T-4.3: RTM FR-23.1
#[test]
fn extract_wikilinks_with_sections_deduplicates_per_section_not_globally() {
    let _guard = TEST_MUTEX.lock();

    let content = "## Section One\n\nReferences [[shared-concept]] here.\n\n## Section Two\n\nAlso references [[shared-concept]] here and again [[shared-concept]].\n\n## Section Three\n\nOnly [[unique-concept]] lives here.\n";
    let concepts = extract_wikilinks_with_sections(content);
    let shared_sections = concepts
        .iter()
        .filter(|(concept, _)| concept == "shared-concept")
        .map(|(_, section_index)| *section_index)
        .collect::<Vec<_>>();
    let unique_sections = concepts
        .iter()
        .filter(|(concept, _)| concept == "unique-concept")
        .map(|(_, section_index)| *section_index)
        .collect::<Vec<_>>();

    assert_eq!(shared_sections, vec![1, 2]);
    assert_eq!(unique_sections, vec![3]);
}

// T-4.3: RTM FR-23.1
#[test]
fn extract_wikilinks_with_sections_reads_wikilinks_inside_h2_headings() {
    let _guard = TEST_MUTEX.lock();

    let content = "## Section about [[heading-concept]]\n\nBody text with [[body-concept]].\n\n## Another [[second-heading]] section\n\nMore body with [[second-body]].\n";
    let concepts = extract_wikilinks_with_sections(content);
    let heading_sections = concepts.into_iter().collect::<BTreeMap<_, _>>();

    assert_eq!(heading_sections.get("heading-concept"), Some(&1));
    assert_eq!(heading_sections.get("body-concept"), Some(&1));
    assert_eq!(heading_sections.get("second-heading"), Some(&2));
    assert_eq!(heading_sections.get("second-body"), Some(&2));
}

// T-4.3: RTM FR-23.2
#[test]
fn sync_same_section_pairs_store_weight_one() {
    let env = TestEnv::isolated();

    let uri = write_memory_file(
        &env.cfg,
        "projects/same-section.md",
        "# Same Section\n\n## Identity\nPair [[alpha]] with [[beta]] in the same section.\n",
    );

    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    let conn = open_db(&env.cfg);
    assert_approx_eq(relation_weight_for_uri(&conn, "alpha", "beta", &uri), 1.0);
}

// T-4.3: RTM FR-23.2
#[test]
fn sync_cross_section_pairs_store_weight_half() {
    let env = TestEnv::isolated();

    let uri = write_memory_file(
        &env.cfg,
        "projects/cross-section.md",
        "# Cross Section\n\n## Identity\nFirst [[alpha]] appears here.\n\n## Sessions\nThen [[beta]] appears in a different section.\n",
    );

    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    let conn = open_db(&env.cfg);
    assert_approx_eq(relation_weight_for_uri(&conn, "alpha", "beta", &uri), 0.5);
}

// T-4.3: RTM FR-23.2
#[test]
fn sync_flat_docs_store_half_weight_for_all_pairs() {
    let env = TestEnv::isolated();

    let uri = write_memory_file(
        &env.cfg,
        "projects/flat-doc.md",
        "# No H2\n\nInline concepts [[x]], [[y]], and [[z]] all live in the preamble.\n",
    );

    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    let conn = open_db(&env.cfg);
    assert_approx_eq(relation_weight_for_uri(&conn, "x", "y", &uri), 0.5);
    assert_approx_eq(relation_weight_for_uri(&conn, "x", "z", &uri), 0.5);
    assert_approx_eq(relation_weight_for_uri(&conn, "y", "z", &uri), 0.5);
}

// T-4.3: RTM FR-23.2
#[test]
fn sync_prefers_named_section_overlap_over_preamble_overlap() {
    let env = TestEnv::isolated();

    let uri = write_memory_file(
        &env.cfg,
        "projects/preamble-and-section.md",
        "# Mixed\n\nPreamble mention [[multi]].\n\n## First Section\n\nSection mentions [[multi]] and [[peer]].\n",
    );

    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    let conn = open_db(&env.cfg);
    assert_approx_eq(relation_weight_for_uri(&conn, "multi", "peer", &uri), 1.0);
}

// T-4.3: RTM FR-23.2
#[test]
fn sync_serializes_source_files_as_weight_dictionary() {
    let env = TestEnv::isolated();

    let uri = write_memory_file(
        &env.cfg,
        "projects/dict-format.md",
        "# Dict\n\n## Identity\nThis file links [[alpha]] and [[beta]].\n",
    );

    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    let conn = open_db(&env.cfg);
    let source_files = relation_source_files(&conn, "alpha", "beta");

    assert!(
        source_files.is_object(),
        "source_files must be a json object: {source_files}"
    );
    assert_approx_eq(source_files[&uri].as_f64().expect("dictionary weight"), 1.0);
}

// T-4.3: RTM FR-23.1 — R1: H3 content attributed to surrounding H2 section
#[test]
fn extract_wikilinks_with_sections_ignores_h3_headings() {
    let _guard = TEST_MUTEX.lock();

    let content = "# Document\n\n## Main Section\n\nIntro mentions [[top-level]].\n\n### Subsection A\n\nDeep content with [[nested-concept]] and [[another-nested]].\n\n## Second Section\n\nBack to H2 with [[second-level]].\n";
    let concepts = extract_wikilinks_with_sections(content);
    let lookup = concepts.into_iter().collect::<BTreeMap<_, _>>();

    // H3 content should be absorbed into the surrounding H2 section (section 1)
    assert_eq!(
        lookup.get("top-level"),
        Some(&1),
        "top-level concept should be in section 1"
    );
    assert_eq!(
        lookup.get("nested-concept"),
        Some(&1),
        "H3 nested concept should stay in parent H2 section 1"
    );
    assert_eq!(
        lookup.get("another-nested"),
        Some(&1),
        "H3 nested concept should stay in parent H2 section 1"
    );
    assert_eq!(
        lookup.get("second-level"),
        Some(&2),
        "second H2 section concept should be in section 2"
    );
}

// T-4.3: RTM FR-23.1 — R2: empty and title-only content returns empty vec
#[test]
fn extract_wikilinks_with_sections_empty_content_returns_empty_vec() {
    let _guard = TEST_MUTEX.lock();

    let empty = extract_wikilinks_with_sections("");
    assert!(
        empty.is_empty(),
        "empty string should produce empty vec, got: {empty:?}"
    );

    let title_only = extract_wikilinks_with_sections("# Title\n");
    assert!(
        title_only.is_empty(),
        "title-only content with no concepts should produce empty vec, got: {title_only:?}"
    );
}

// T-4.3: RTM FR-23.2 — R5: same pair across multiple files aggregates weights
#[test]
fn sync_same_pair_across_multiple_files_aggregates_source_weights() {
    let env = TestEnv::isolated();

    let uri_a = write_memory_file(
        &env.cfg,
        "projects/multi-file-a.md",
        "# File A\n\n## Identity\nLinks [[alpha]] with [[beta]] in same section.\n",
    );

    let uri_b = write_memory_file(
        &env.cfg,
        "projects/multi-file-b.md",
        "# File B\n\n## First\nHas [[alpha]] here.\n\n## Second\nHas [[beta]] there.\n",
    );

    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    let conn = open_db(&env.cfg);
    let source_files = relation_source_files(&conn, "alpha", "beta");

    assert!(
        source_files.is_object(),
        "source_files must be a json object: {source_files}"
    );

    // File A: same section → weight 1.0
    assert_approx_eq(
        source_files[&uri_a]
            .as_f64()
            .expect("file A weight should be present"),
        1.0,
    );

    // File B: cross section → weight 0.5
    assert_approx_eq(
        source_files[&uri_b]
            .as_f64()
            .expect("file B weight should be present"),
        0.5,
    );
}
