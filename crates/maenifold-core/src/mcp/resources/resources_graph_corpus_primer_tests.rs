use super::resources_graph_corpus_primer_test_support::*;
use super::resources_tests_common::*;

#[test]
#[serial]
fn graph_corpus_primer_fixture_payload_is_deterministic() {
    let (_tmp, _guard) = setup_primer_fixture();

    let first =
        extract_text(handle_read_resource(PRIMER_URI).expect("primer resource should be readable"));
    let second = extract_text(
        handle_read_resource(PRIMER_URI).expect("primer resource should be readable twice"),
    );

    assert_eq!(
        first, second,
        "primer output must be byte-stable for the same fixture"
    );
    assert_payload_lines(&first, &expected_lines());
}

#[test]
#[serial]
fn graph_corpus_primer_limit_query_prefers_fresh_community() {
    let (_tmp, _guard) = setup_primer_fixture();

    let payload = extract_text(
        handle_read_resource(PRIMER_LIMIT_ONE_URI)
            .expect("primer resource with limit query should be readable"),
    );

    assert_payload_lines(&payload, &[expected_lines()[0]]);
}

#[test]
#[serial]
fn graph_corpus_primer_empty_graph_returns_empty_lines_payload() {
    let (_tmp, _guard) = setup_empty_primer_fixture();

    let payload = extract_text(
        handle_read_resource(PRIMER_URI).expect("primer resource should handle empty graphs"),
    );

    assert_payload_lines(&payload, &[]);
}

#[test]
#[serial]
fn graph_corpus_primer_large_fixture_stays_under_eight_kib() {
    let (_tmp, _guard) = setup_large_primer_fixture();

    let payload = extract_text(
        handle_read_resource(PRIMER_URI).expect("primer resource should render large fixtures"),
    );

    let bytes = payload.as_bytes().len();
    assert!(
        bytes <= 8 * 1024,
        "primer payload exceeded 8KiB hard cap: {bytes} bytes"
    );
}
