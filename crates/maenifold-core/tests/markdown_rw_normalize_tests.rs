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

#[test]
fn normalize_concept_multiple_slashes_compresses_to_single_hyphen() {
    assert_eq!(normalize_concept("auth / / token"), "auth-token");
}

#[test]
fn normalize_concept_mixed_separators_compresses_to_single_hyphen() {
    assert_eq!(normalize_concept("API  __  //  Gateway"), "api-gateway");
}

#[test]
fn normalize_concept_leading_trailing_separators_trims_hyphens() {
    assert_eq!(normalize_concept("  concept  "), "concept");
}

#[test]
fn normalize_concept_only_separators_returns_empty() {
    assert_eq!(normalize_concept("   ___  ///  "), "");
}

#[test]
fn normalize_concept_single_separators_works_as_expected() {
    assert_eq!(normalize_concept("REST API Gateway"), "rest-api-gateway");
}

#[test]
fn normalize_concept_no_separators_works_as_expected() {
    assert_eq!(normalize_concept("Authentication"), "authentication");
}

#[test]
fn normalize_concept_triple_hyphens_compresses_to_single_hyphen() {
    assert_eq!(normalize_concept("test---concept"), "test-concept");
}

#[test]
fn normalize_concept_many_consecutive_hyphens_compresses_to_single_hyphen() {
    assert_eq!(normalize_concept("test----------concept"), "test-concept");
}
