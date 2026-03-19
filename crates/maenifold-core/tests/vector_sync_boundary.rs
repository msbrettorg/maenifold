use maenifold_core::json::OutputFormat;
use maenifold_core::{Config, ToolRegistry};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::json;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

// T-12.7: RTM NFR-1 — acceptance hygiene trims smoke-only sync boundary tests that under-assert runtime behavior.

fn isolated_root() -> (TempDir, Config) {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    (tmp, cfg)
}

// T-12.7: REMOVED — vector_sync_tools_are_registered_case_insensitively
// Zero unique assertions; fully subsumed by tool_registry_tests.rs which exhaustively
// tests all 29 tools including case-insensitive invocation with arbitrary mixed-case forms.

// T-12.7: MERGED — markdown_reader_sync_should_match_single_pass_extraction_contract
// Moved to markdown_rw_sync_tests.rs as sync_extract_excludes_wikilinks_in_all_code_contexts.
// Original was duplicated at unit, sync, and CLI integration layers.

#[test]
fn find_similar_concepts_after_sync_surfaces_co_occurring_concepts() {
    // T-12.7: RTM NFR-1 — REWRITE: renamed from find_similar_concepts_after_sync_should_surface_ranked_related_concepts.
    // Strengthened: structured JSON assertions on result format, score bounds, rank ordering,
    // and maxResults enforcement. Added asymmetric seed data to verify ranking.
    let _guard = TEST_MUTEX.lock();
    let (_tmp, _cfg) = isolated_root();
    let registry = ToolRegistry::global();

    // Seed: strong double co-occurrence for oauth2, single for jwt
    for payload in [
        json!({"title":"OAuth A","content":"Links [[authentication]] to [[oauth2]]."}),
        json!({"title":"OAuth B","content":"Links [[authentication]] to [[oauth2]]."}),
        json!({"title":"JWT A","content":"Links [[authentication]] to [[jwt]]."}),
    ] {
        let w = registry
            .try_invoke("WriteMemory", &payload, OutputFormat::Json)
            .expect("write seed");
        let p: serde_json::Value = serde_json::from_str(&w).expect("write json");
        assert_eq!(p["success"], true, "seed write should succeed: {w}");
    }

    // Sync returns markdown regardless of output format; check for completion marker
    let sync = registry
        .try_invoke("Sync", &json!({}), OutputFormat::Markdown)
        .expect("sync invoke");
    assert!(
        sync.contains("Sync complete:"),
        "sync should complete before semantic lookup: {sync}"
    );
    assert!(
        !sync.contains("ERROR"),
        "sync should not contain errors: {sync}"
    );

    let output = registry
        .try_invoke(
            "FindSimilarConcepts",
            &json!({"conceptName":"authentication","maxResults":3}),
            OutputFormat::Json,
        )
        .expect("find similar invoke");
    let parsed: serde_json::Value = serde_json::from_str(&output).expect("json parse");

    assert_eq!(
        parsed["success"], true,
        "FindSimilarConcepts should succeed: {output}"
    );

    // Results list is present and bounded by maxResults
    let results = parsed["data"]["results"]
        .as_array()
        .expect("results should be an array");
    assert!(
        results.len() <= 3,
        "maxResults=3 not respected, got {} results: {output}",
        results.len()
    );

    // Both co-occurring concepts are surfaced
    let names: Vec<&str> = results.iter().filter_map(|r| r["name"].as_str()).collect();
    assert!(
        names.contains(&"oauth2"),
        "expected oauth2 in results: {names:?}"
    );
    assert!(names.contains(&"jwt"), "expected jwt in results: {names:?}");

    // Similarity scores are bounded to (0, 1]
    for result in results {
        let similarity = result["similarity"]
            .as_f64()
            .expect("similarity should be a number");
        assert!(
            similarity > 0.0 && similarity <= 1.0,
            "similarity must be in (0, 1], got {similarity}: {output}"
        );
    }

    // Stronger co-occurrence (oauth2 × 2 edges) ranks above weaker (jwt × 1 edge)
    let oauth2_pos = names.iter().position(|n| *n == "oauth2").unwrap();
    let jwt_pos = names.iter().position(|n| *n == "jwt").unwrap();
    assert!(
        oauth2_pos < jwt_pos,
        "oauth2 (2 co-occurrences) should rank above jwt (1 co-occurrence): {names:?}"
    );

    Config::reset_overrides();
}
