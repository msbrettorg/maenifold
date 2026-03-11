// T-12.0: RTM NFR-1
#[path = "proximity_weight/support.rs"]
mod support;

use maenifold_core::markdown::extract_wikilinks_with_sections;
use std::collections::BTreeMap;
use support::{
    TestEnv, acquire_test_lock, assert_approx_eq, invoke_sync, open_db, relation_source_files,
    relation_weight_for_uri, write_memory_file,
};

// T-4.3: RTM FR-23.1
#[test]
fn extract_wikilinks_with_sections_tags_multi_section_documents() {
    let _guard = acquire_test_lock();

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
    let _guard = acquire_test_lock();

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
    let _guard = acquire_test_lock();

    let content = "# Document\n\nThis preamble mentions [[preamble-concept]] before any heading.\n\n## First Section\n\nThis section has [[section-concept]].\n";
    let concepts = extract_wikilinks_with_sections(content);
    let lookup = concepts.into_iter().collect::<BTreeMap<_, _>>();

    assert_eq!(lookup.get("preamble-concept"), Some(&0));
    assert_eq!(lookup.get("section-concept"), Some(&1));
}

// T-4.3: RTM FR-23.1
#[test]
fn extract_wikilinks_with_sections_deduplicates_per_section_not_globally() {
    let _guard = acquire_test_lock();

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
    let _guard = acquire_test_lock();

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
    let _guard = acquire_test_lock();
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
    let _guard = acquire_test_lock();
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
    let _guard = acquire_test_lock();
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
    let _guard = acquire_test_lock();
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
    let _guard = acquire_test_lock();
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
