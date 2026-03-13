#[path = "sync_wikilink_filter/support.rs"]
mod sync_wikilink_filter_support;

use maenifold_core::markdown::normalize_concept;
use sync_wikilink_filter_support::*;

// T-22.4: RTM-151
#[test]
fn sync_filters_exact_blocked_concepts() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let blocked = normalize_concept("agent");
    let allowed = normalize_concept("tool");
    let (_path, file_uri) = create_test_markdown_file(
        &cfg,
        "exact-blocks.md",
        "# Exact Blocks\n\nReferences [[agent]] and [[tool]].\n",
    );
    write_wikilink_filter(&cfg, r#"{"agent":"noise concept"}"#);

    let output = sync_all(&cfg);

    assert!(
        output.contains("Sync complete:"),
        "full sync should return a summary: {output}"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &blocked),
        None,
        "blocked exact concept must not be inserted into concept_mentions"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &allowed),
        Some(1),
        "allowed concept must still be inserted into concept_mentions"
    );
}

// T-22.4: RTM-152
#[test]
fn sync_filters_prefix_blocked_concepts() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let blocked_task = normalize_concept("T-14.1");
    let blocked_rtm = normalize_concept("RTM-042");
    let allowed = normalize_concept("real-concept");
    let (_path, file_uri) = create_test_markdown_file(
        &cfg,
        "prefix-blocks.md",
        "# Prefix Blocks\n\nReferences [[T-14.1]], [[RTM-042]], and [[real-concept]].\n",
    );
    write_wikilink_filter(&cfg, r#"{"T-*":"task IDs","RTM-*":"traceability IDs"}"#);

    sync_all(&cfg);

    assert_eq!(
        get_mention_count(&cfg, &file_uri, &blocked_task),
        None,
        "task-ID prefix matches must be filtered from concept_mentions"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &blocked_rtm),
        None,
        "RTM-ID prefix matches must be filtered from concept_mentions"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &allowed),
        Some(1),
        "non-matching concepts must still be recorded"
    );
}

// T-22.4: RTM-153
#[test]
fn sync_output_reports_filtered_count() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    create_test_markdown_file(
        &cfg,
        "output-count-a.md",
        "# Output Count A\n\nReferences [[agent]], [[tool]], and [[real-concept]].\n",
    );
    create_test_markdown_file(
        &cfg,
        "output-count-b.md",
        "# Output Count B\n\nReferences [[T-14.1]] and [[kept-concept]].\n",
    );
    write_wikilink_filter(
        &cfg,
        r#"{"agent":"noise concept","tool":"hub concept","T-*":"task IDs"}"#,
    );

    let output = sync_all(&cfg);
    let lower = output.to_ascii_lowercase();
    let filtered_line = lower
        .lines()
        .find(|line| line.contains("filtered"))
        .unwrap_or_else(|| panic!("sync output must mention filtered concepts: {output}"));

    assert!(
        filtered_line.chars().any(|ch| matches!(ch, '1'..='9')),
        "filtered summary line must report a non-zero filtered count: {filtered_line}"
    );
}

// T-22.4: RTM-154
#[test]
fn incremental_sync_also_filters() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let blocked = normalize_concept("blocked-concept");
    let allowed = normalize_concept("allowed-concept");
    let (path, file_uri) = create_test_markdown_file(
        &cfg,
        "incremental.md",
        "# Incremental\n\nReferences [[blocked-concept]] and [[allowed-concept]].\n",
    );
    write_wikilink_filter(&cfg, r#"{"blocked-concept":"explicitly blocked"}"#);

    let output = sync_one(&cfg, &path);

    assert!(
        output.contains("Synced memory://tests/sync-wikilink-filter/incremental"),
        "incremental sync should process the fixture file: {output}"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &blocked),
        None,
        "incremental sync must not insert blocked concepts into concept_mentions"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &allowed),
        Some(1),
        "incremental sync must still record allowed concepts"
    );
}

// T-22.4: RTM-155
#[test]
fn sync_works_without_filter_file() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let concept_a = normalize_concept("agent");
    let concept_b = normalize_concept("tool");
    let (_path, file_uri) = create_test_markdown_file(
        &cfg,
        "no-filter.md",
        "# No Filter\n\nReferences [[agent]] and [[tool]].\n",
    );

    let output = sync_all(&cfg);

    assert!(
        output.contains("Sync complete:"),
        "missing filter file must not break sync: {output}"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &concept_a),
        Some(1),
        "without a filter file, exact concepts must remain unfiltered"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &concept_b),
        Some(1),
        "without a filter file, additional concepts must remain unfiltered"
    );
}

// T-22.4: RTM-156
#[test]
fn sync_with_filter_produces_fewer_concepts() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    create_test_markdown_file(
        &cfg,
        "count-a.md",
        "# Count A\n\nReferences [[agent]], [[allowed-one]], and [[T-14.1]].\n",
    );
    create_test_markdown_file(
        &cfg,
        "count-b.md",
        "# Count B\n\nReferences [[tool]], [[allowed-two]], and [[RTM-042]].\n",
    );

    sync_all(&cfg);
    let count_without_filter = count_all_concepts(&cfg);

    reset_database(&cfg);
    write_wikilink_filter(
        &cfg,
        r#"{"agent":"noise concept","tool":"hub concept","T-*":"task IDs","RTM-*":"traceability IDs"}"#,
    );

    sync_all(&cfg);
    let count_with_filter = count_all_concepts(&cfg);

    assert!(
        count_without_filter > 0,
        "precondition: unfiltered sync should create concept rows"
    );
    assert!(
        count_with_filter < count_without_filter,
        "sync with filter should produce fewer concepts than sync without filter \
         (without={count_without_filter}, with={count_with_filter})"
    );
}

// T-22.5: RTM-152
#[test]
fn sync_prefix_filter_does_not_block_unrelated_concepts() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let blocked = normalize_concept("T-14.1");
    let allowed_tool = normalize_concept("tool");
    let allowed_template = normalize_concept("template");
    let (_path, file_uri) = create_test_markdown_file(
        &cfg,
        "prefix-false-positive.md",
        "# Prefix Guard\n\nReferences [[T-14.1]], [[tool]], and [[template]].\n",
    );
    write_wikilink_filter(&cfg, r#"{"T-*":"task IDs"}"#);

    sync_all(&cfg);

    assert_eq!(
        get_mention_count(&cfg, &file_uri, &blocked),
        None,
        "T-14.1 must be blocked by T-* prefix"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &allowed_tool),
        Some(1),
        "tool must NOT be blocked by T-* prefix — only 't-' prefix should match"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &allowed_template),
        Some(1),
        "template must NOT be blocked by T-* prefix — only 't-' prefix should match"
    );
}

// T-22.5: RTM-156
#[test]
fn sync_filter_excludes_blocked_from_concept_graph() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let blocked = normalize_concept("noise-concept");
    let allowed_a = normalize_concept("real-a");
    let allowed_b = normalize_concept("real-b");
    create_test_markdown_file(
        &cfg,
        "graph-exclude.md",
        "# Graph Exclude\n\nReferences [[noise-concept]], [[real-a]], and [[real-b]].\n",
    );
    write_wikilink_filter(&cfg, r#"{"noise-concept":"test noise"}"#);

    sync_all(&cfg);

    let graph_count_blocked = count_concept_graph_entries(&cfg, &blocked);
    let graph_count_allowed = count_concept_graph_entries(&cfg, &allowed_a);

    assert_eq!(
        graph_count_blocked, 0,
        "blocked concept must have zero rows in concept_graph"
    );
    assert!(
        graph_count_allowed > 0,
        "allowed concepts must have relations in concept_graph"
    );

    let _ = allowed_b;
}

// T-22.5: RTM-155
#[test]
fn sync_does_not_crash_with_malformed_filter_file() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();

    let concept = normalize_concept("agent");
    let (_path, file_uri) = create_test_markdown_file(
        &cfg,
        "malformed-filter.md",
        "# Malformed Filter\n\nReferences [[agent]] and [[tool]].\n",
    );
    write_wikilink_filter(&cfg, "not valid json {{{");

    let output = sync_all(&cfg);

    assert!(
        output.contains("Sync complete:"),
        "sync must complete despite malformed filter file: {output}"
    );
    assert_eq!(
        get_mention_count(&cfg, &file_uri, &concept),
        Some(1),
        "with malformed filter, all concepts should pass through unfiltered"
    );
}
