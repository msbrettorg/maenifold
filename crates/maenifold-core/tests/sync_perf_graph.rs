// T-0.1: RTM NFR-3
#![allow(dead_code, unused_imports)]

include!("sync_perf_support.rs");

// T-0.1: RTM NFR-3
#[test]
fn build_graph_edges_creates_expected_edges_with_correct_source_files_json() {
    let env = TestEnv::new();
    let unique = unique_suffix();
    let concept_a = normalize_concept(&format!("edge-alpha-{unique}"));
    let concept_b = normalize_concept(&format!("edge-beta-{unique}"));
    let concept_c = normalize_concept(&format!("edge-gamma-{unique}"));
    let relative_path = format!("edge-tests/edge-note-{unique}.md");
    let uri = write_memory_fixture(
        &env.cfg,
        &relative_path,
        &format!(
            "# Edge Test\n\n## Shared Section\n\nMentions [[{concept_a}]], [[{concept_b}]], and [[{concept_c}]].\n"
        ),
    );

    let output = sync_one(&env, &relative_path);
    assert!(output.contains("affected relations rebuilt"), "{output}");

    for (left, right) in [
        ordered_pair(concept_a.as_str(), concept_b.as_str()),
        ordered_pair(concept_a.as_str(), concept_c.as_str()),
        ordered_pair(concept_b.as_str(), concept_c.as_str()),
    ] {
        let (count, source_files) = query_edge(&env.cfg, left, right)
            .unwrap_or_else(|| panic!("missing edge ({left}, {right})"));
        // T-0.1: RTM NFR-3 — co_occurrence_count = number of distinct source files (1 here).
        assert_eq!(
            count, 1.0,
            "co_occurrence_count should be 1.0 (one contributing file) for ({left}, {right})"
        );
        assert_eq!(source_files.get(&uri).copied(), Some(1.0));
    }
}

// T-0.1: RTM NFR-3
#[test]
fn build_graph_edges_proximity_weight_same_section_vs_cross_section() {
    let env = TestEnv::new();
    let unique = unique_suffix();
    let concept_a = normalize_concept(&format!("prox-alpha-{unique}"));
    let concept_b = normalize_concept(&format!("prox-beta-{unique}"));
    let concept_c = normalize_concept(&format!("prox-gamma-{unique}"));
    let relative_path = format!("prox-tests/prox-note-{unique}.md");
    let uri = write_memory_fixture(
        &env.cfg,
        &relative_path,
        &format!(
            "# Proximity Test\n\nPreamble: [[{concept_a}]] and [[{concept_c}]].\n\n## Section One\n\n[[{concept_b}]] and [[{concept_c}]] here.\n"
        ),
    );

    let output = sync_one(&env, &relative_path);
    assert!(output.contains("affected relations rebuilt"), "{output}");

    assert_eq!(
        relation_weight_for_uri(&env.cfg, &concept_a, &concept_b, &uri),
        0.5
    );
    // Preamble (section_index=0) co-occurrence is intentionally treated as cross-section
    // per T-4.3 / FR-23.2: flat docs keep section_index=0 for extraction, but sync treats
    // that implicit root as cross-section, yielding 0.5 instead of 1.0.
    assert_eq!(
        relation_weight_for_uri(&env.cfg, &concept_a, &concept_c, &uri),
        0.5
    );
    assert_eq!(
        relation_weight_for_uri(&env.cfg, &concept_b, &concept_c, &uri),
        1.0
    );
}

// T-0.1: RTM NFR-3
// This test owns "no duplicate rows + correct co_occurrence_count" for a single same-section pair.
// See also: concept_sync_edge_stability.rs — owns multi-edge rowid preservation across section types.
#[test]
fn build_graph_edges_resync_is_true_upsert_rowid_stable_and_weight_not_doubled() {
    let env = TestEnv::new();
    let unique = unique_suffix();
    let concept_a = normalize_concept(&format!("resync-alpha-{unique}"));
    let concept_b = normalize_concept(&format!("resync-beta-{unique}"));
    let relative_path = format!("reg-resync/resync-note-{unique}.md");
    let uri = write_memory_fixture(
        &env.cfg,
        &relative_path,
        &format!("# Resync Test\n\n## Shared\n\n[[{concept_a}]] and [[{concept_b}]] here.\n"),
    );

    let first = sync_one(&env, &relative_path);
    assert!(first.contains("affected relations rebuilt"), "{first}");
    let rowid_before = query_edge_rowid(&env.cfg, &concept_a, &concept_b).expect("edge rowid");

    set_last_indexed(&env.cfg, &uri, "2000-01-01T00:00:00+00:00");
    write_memory_fixture(
        &env.cfg,
        &relative_path,
        &format!(
            "# Resync Test Updated\n\n## Shared\n\n[[{concept_a}]] and [[{concept_b}]] still here.\n"
        ),
    );

    let second = sync_one(&env, &relative_path);
    assert!(second.contains("affected relations rebuilt"), "{second}");
    assert_eq!(query_edge_row_count(&env.cfg, &concept_a, &concept_b), 1);
    assert_eq!(
        query_edge_rowid(&env.cfg, &concept_a, &concept_b),
        Some(rowid_before)
    );
    assert_eq!(
        relation_weight_for_uri(&env.cfg, &concept_a, &concept_b, &uri),
        1.0
    );
}

// T-0.1: RTM NFR-3
#[test]
fn build_graph_edges_with_over_1000_concepts_creates_edges_without_error() {
    let env = TestEnv::new();
    let unique = unique_suffix();
    let relative_path = format!("perf-bt-002/large-concept-file-{unique}.md");
    let concept_names = (0..1100)
        .map(|index| format!("perf-bt-002-concept-{index:04}-{unique}"))
        .collect::<Vec<_>>();

    let mut content = String::from("# Large Concept File\n\n## Shared\n\n");
    for concept in &concept_names {
        content.push_str(&format!("[[{concept}]] "));
    }
    content.push('\n');
    write_memory_fixture(&env.cfg, &relative_path, &content);

    let output = sync_one(&env, &relative_path);
    assert!(output.contains("affected relations rebuilt"), "{output}");

    // C(1100, 2) = 604_450
    let expected_edges: i64 = (1100 * 1099) / 2;
    let edge_count = open_db(&env.cfg)
        .query_row("SELECT COUNT(*) FROM concept_graph", [], |row| {
            row.get::<_, i64>(0)
        })
        .expect("edge count");
    assert_eq!(
        edge_count, expected_edges,
        "concept_graph should contain all C(1100,2) = {expected_edges} edges; got {edge_count}"
    );

    let first = normalize_concept(&concept_names[0]);
    let second = normalize_concept(&concept_names[1]);
    let penultimate = normalize_concept(&concept_names[1098]);
    let last = normalize_concept(&concept_names[1099]);
    assert_eq!(query_edge_row_count(&env.cfg, &first, &second), 1);
    assert_eq!(query_edge_row_count(&env.cfg, &penultimate, &last), 1);
}
