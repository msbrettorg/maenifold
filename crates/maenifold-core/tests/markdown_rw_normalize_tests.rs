use maenifold_core::markdown::normalize_concept;

// T-12.7.1: RTM NFR-1 — Split markdown normalization regressions into focused coverage.

#[test]
fn normalize_concept_double_space_compresses_to_single_hyphen() {
    assert_eq!(normalize_concept("API  Gateway"), "api-gateway");
}

#[test]
fn normalize_concept_double_underscore_compresses_to_single_hyphen() {
    assert_eq!(normalize_concept("user__profile"), "user-profile");
}

// T-12.7.1: RTM NFR-1 — Renamed: input contains spaces AND slashes, not slashes alone.
#[test]
fn normalize_concept_slash_and_space_mix_deduplicates_to_single_hyphen() {
    assert_eq!(normalize_concept("auth / / token"), "auth-token");
}

#[test]
fn normalize_concept_mixed_separators_compresses_to_single_hyphen() {
    assert_eq!(normalize_concept("API  __  //  Gateway"), "api-gateway");
}

// T-12.7.1: RTM NFR-1 — Renamed: input is leading/trailing whitespace that maps to hyphens then trims.
#[test]
fn normalize_concept_leading_trailing_spaces_produce_trimmed_result() {
    assert_eq!(normalize_concept("  concept  "), "concept");
}

#[test]
fn normalize_concept_only_separators_returns_empty() {
    assert_eq!(normalize_concept("   ___  ///  "), "");
}

// T-12.7.1: RTM NFR-1 — Renamed: describes the actual property (single spaces become hyphens, lowercase).
#[test]
fn normalize_concept_single_spaces_produce_hyphen_delimited_lowercase() {
    assert_eq!(normalize_concept("REST API Gateway"), "rest-api-gateway");
}

// T-12.7.1: RTM NFR-1 — Renamed: describes the pure-lowercase path with no separator logic.
#[test]
fn normalize_concept_no_separators_returns_lowercased_input() {
    assert_eq!(normalize_concept("Authentication"), "authentication");
}

#[test]
fn normalize_concept_triple_hyphens_compresses_to_single_hyphen() {
    assert_eq!(normalize_concept("test---concept"), "test-concept");
}

// T-12.7.1: RTM NFR-1 — Pin empty-input edge case: empty WikiLink [[]] produces "".
#[test]
fn normalize_concept_empty_input_returns_empty() {
    assert_eq!(normalize_concept(""), "");
}

// T-12.7.1: RTM NFR-1 — Pin passthrough behavior: digits and pre-existing hyphens preserved.
#[test]
fn normalize_concept_digits_and_passthrough_chars_preserved() {
    assert_eq!(normalize_concept("user-123"), "user-123");
}
