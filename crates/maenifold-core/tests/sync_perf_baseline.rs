// T-0.1: RTM NFR-3
#![allow(dead_code, unused_imports)]

include!("sync_perf_support.rs");

// T-0.1: RTM NFR-3
#[test]
fn baseline_process_file_timing_per_file() {
    let env = TestEnv::new();
    let mut per_file_ms = Vec::new();

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

        let started = Instant::now();
        let output = sync_one(&env, &relative_path);
        per_file_ms.push(started.elapsed().as_millis() as u64);
        assert!(
            output.contains("Synced memory://perf-baseline/"),
            "single-file sync should succeed: {output}"
        );
    }

    per_file_ms.sort_unstable();
    let total: u64 = per_file_ms.iter().sum();
    let median = per_file_ms[per_file_ms.len() / 2];
    let max = *per_file_ms.last().expect("max sample");
    eprintln!(
        "PROCESS-FILE BASELINE files=10 total_ms={total} median_ms={median} max_ms={max} samples={per_file_ms:?}"
    );

    assert_eq!(file_content_row_count(&env.cfg), 10);
}

// T-0.1: RTM NFR-3
#[test]
fn baseline_full_sync_timing_over_corpus() {
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

    let started = Instant::now();
    let output = sync_all(&env);
    let elapsed_ms = started.elapsed().as_millis();
    eprintln!("FULL-SYNC BASELINE files=10 elapsed_ms={elapsed_ms}\n{output}");

    assert!(
        output.contains("Sync complete:"),
        "unexpected sync output: {output}"
    );
    assert!(
        output.contains("files processed") || output.contains("FILES processed"),
        "sync output should report processed files: {output}"
    );
}

// T-0.1: RTM NFR-3
#[test]
fn count_concept_occurrences_golden_alpha_two_beta_one() {
    let content = "[[alpha]] and [[beta]] appear in [[alpha]] again";
    assert_eq!(count_visible_wikilink_occurrences(content, "alpha"), 2);
    assert_eq!(count_visible_wikilink_occurrences(content, "beta"), 1);
}

// T-0.1: RTM NFR-3
#[test]
fn extract_wikilinks_with_sections_group_by_count_documents_relation_to_count_occurrences() {
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
fn count_concept_occurrences_across_sections_counts_all_occurrences() {
    let content = "Preamble mentions [[gamma]].\n\n## Section One\n\n[[gamma]] and [[delta]] here.\n\n## Section Two\n\n[[gamma]] appears a third time.\n";
    assert_eq!(count_visible_wikilink_occurrences(content, "gamma"), 3);
    assert_eq!(count_visible_wikilink_occurrences(content, "delta"), 1);
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

// T-0.1: RTM NFR-3
#[test]
fn count_all_concept_occurrences_matches_per_concept_counts_for_all_concepts() {
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
