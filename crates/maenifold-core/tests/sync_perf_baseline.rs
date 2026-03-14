// T-0.1: RTM NFR-3
#![allow(dead_code, unused_imports)]

include!("sync_perf_support.rs");

// T-0.1: RTM NFR-3 — REWRITE: stripped dead timing instrumentation, renamed as functional test.
#[test]
fn sync_single_file_indexes_all_files_in_corpus() {
    let env = TestEnv::new();

    for index in 0..10 {
        let unique = unique_suffix();
        let relative_path = format!("perf-baseline/note-{index:02}-{unique}.md");
        write_memory_fixture(
            &env.cfg,
            &relative_path,
            &format!(
                "# Note {index}\n\nReferences [[concept-a-{unique}]] and [[concept-b-{unique}]].\nAlso mentions [[concept-a-{unique}]] again for count=2.\n"
            ),
        );

        let output = sync_one(&env, &relative_path);
        assert!(
            output.contains("Synced memory://perf-baseline/"),
            "single-file sync should succeed: {output}"
        );
    }

    assert_eq!(file_content_row_count(&env.cfg), 10);
}

// T-0.1: RTM NFR-3 — REWRITE: stripped dead timing instrumentation, renamed as functional test.
#[test]
fn sync_all_completes_over_ten_file_corpus() {
    let env = TestEnv::new();

    for index in 0..10 {
        let unique = unique_suffix();
        let relative_path = format!("perf-sync-corpus/corpus-{index:02}-{unique}.md");
        write_memory_fixture(
            &env.cfg,
            &relative_path,
            &format!(
                "# Corpus Note {index}\n\nMentions [[sync-concept-{unique}]] and [[shared-concept]].\nSecond mention of [[sync-concept-{unique}]] for count testing.\n"
            ),
        );
    }

    let output = sync_all(&env);

    assert!(
        output.contains("Sync complete:"),
        "unexpected sync output: {output}"
    );
    assert!(
        output.contains("files processed") || output.contains("FILES processed"),
        "sync output should report processed files: {output}"
    );
}

// T-0.1: RTM NFR-3 — MERGE of tests 3+5: helper_count_visible_wikilinks covers single- and
// multi-section content in a single test. Tests the local oracle, not production code.
#[test]
fn helper_count_visible_wikilinks_single_and_multi_section() {
    // Single-section content: alpha appears twice, beta once.
    let single = "[[alpha]] and [[beta]] appear in [[alpha]] again";
    assert_eq!(count_visible_wikilink_occurrences(single, "alpha"), 2);
    assert_eq!(count_visible_wikilink_occurrences(single, "beta"), 1);

    // Multi-section content: gamma appears in preamble + two sections, delta in one section.
    let multi = "Preamble mentions [[gamma]].\n\n## Section One\n\n[[gamma]] and [[delta]] here.\n\n## Section Two\n\n[[gamma]] appears a third time.\n";
    assert_eq!(count_visible_wikilink_occurrences(multi, "gamma"), 3);
    assert_eq!(count_visible_wikilink_occurrences(multi, "delta"), 1);
}

// T-0.1: RTM NFR-3 — KEEP (renamed): documents that extract_wikilinks_with_sections returns one
// entry per concept per section (deduplicated), unlike the raw occurrence counter.
#[test]
fn extract_wikilinks_with_sections_deduplicates_per_section_unlike_occurrence_count() {
    let content = "[[alpha]] and [[beta]] appear in [[alpha]] again";
    let concepts_with_sections = extract_wikilinks_with_sections(content);

    let section_counts = concepts_with_sections.into_iter().fold(
        BTreeMap::<String, usize>::new(),
        |mut counts, (concept, _section_index)| {
            *counts.entry(concept).or_default() += 1;
            counts
        },
    );

    assert_eq!(section_counts.get("alpha"), Some(&1));
    assert_eq!(section_counts.get("beta"), Some(&1));
    assert_eq!(count_visible_wikilink_occurrences(content, "alpha"), 2);
    assert_eq!(count_visible_wikilink_occurrences(content, "beta"), 1);
}

// T-0.1: RTM NFR-3
#[test]
fn extract_wikilinks_with_sections_assigns_section_indices_correctly() {
    let content = "Preamble mentions [[gamma]].\n\n## Section One\n\n[[gamma]] and [[delta]] here.\n\n## Section Two\n\n[[gamma]] appears a third time.\n";
    let concepts = extract_wikilinks_with_sections(content);

    let gamma_sections = concepts
        .iter()
        .filter(|(concept, _)| concept == "gamma")
        .map(|(_, section)| *section)
        .collect::<Vec<_>>();
    let delta_sections = concepts
        .iter()
        .filter(|(concept, _)| concept == "delta")
        .map(|(_, section)| *section)
        .collect::<Vec<_>>();

    assert_eq!(gamma_sections, vec![0, 1, 2]);
    assert_eq!(delta_sections, vec![1]);
}

// T-0.1: RTM NFR-3 — REWRITE: renamed with helper_ prefix to signal this tests local oracle,
// not production code. Assertions unchanged.
#[test]
fn helper_count_all_occurrences_matches_per_concept_counts() {
    let content = "# Multi-Count Test\n\n[[alpha]] and [[beta]] start the file.\nThen [[alpha]] again, and [[gamma]].\n\n## Section One\n\n[[alpha]] and [[beta]] in section one.\nInline code `[[alpha]]` should NOT count.\n\n## Section Two\n\n[[gamma]] appears here too.\n[[delta]] only once.\n";
    let concepts = ["alpha", "beta", "gamma", "delta", "epsilon"];

    let expected = concepts
        .iter()
        .map(|concept| {
            (
                (*concept).to_string(),
                count_visible_wikilink_occurrences(content, concept),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let actual = count_all_visible_wikilink_occurrences(content, &concepts);

    assert_eq!(actual, expected);
    assert_eq!(actual.get("alpha"), Some(&3));
    assert_eq!(actual.get("beta"), Some(&2));
    assert_eq!(actual.get("gamma"), Some(&2));
    assert_eq!(actual.get("delta"), Some(&1));
    assert_eq!(actual.get("epsilon"), Some(&0));
}

// T-0.1: RTM NFR-3 — NEW: explicit inline-code exclusion test (P0 recommendation 3).
// Makes the code-stripping boundary of visible_text_without_code a red-test-guarded contract.
#[test]
fn helper_count_visible_wikilinks_skips_inline_code_spans() {
    let content = "[[real]] and `[[fake]]`";
    assert_eq!(count_visible_wikilink_occurrences(content, "real"), 1);
    assert_eq!(count_visible_wikilink_occurrences(content, "fake"), 0);
}

// T-0.1: RTM NFR-3
#[test]
fn count_all_concept_occurrences_returns_zero_for_absent_concepts() {
    let content = "Only [[present-concept]] is here.";
    let counts = count_all_visible_wikilink_occurrences(
        content,
        &["present-concept", "absent-concept", "also-absent"],
    );

    assert_eq!(counts.get("present-concept"), Some(&1));
    assert_eq!(counts.get("absent-concept"), Some(&0));
    assert_eq!(counts.get("also-absent"), Some(&0));
}
