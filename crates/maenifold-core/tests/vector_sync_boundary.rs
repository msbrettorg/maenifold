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

#[test]
fn vector_sync_tools_are_registered_case_insensitively() {
    let registry = ToolRegistry::global();
    for tool in [
        "FindSimilarConcepts",
        "findsimilarconcepts",
        "StartWatcher",
        "startwatcher",
        "StopWatcher",
        "stopwatcher",
        "RunFullBenchmark",
        "runfullbenchmark",
        "Sync",
        "sync",
    ] {
        assert!(registry.contains(tool), "missing tool registration: {tool}");
    }
}

#[test]
fn find_similar_concepts_after_sync_should_surface_ranked_related_concepts() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, _cfg) = isolated_root();

    let registry = ToolRegistry::global();
    for payload in [
        json!({"title":"OAuth Seed","content":"This note links [[authentication]] to [[oauth2]]."}),
        json!({"title":"JWT Seed","content":"This note links [[authentication]] to [[jwt]]."}),
    ] {
        let write = registry
            .try_invoke("WriteMemory", &payload, OutputFormat::Json)
            .expect("write seed");
        let parsed: serde_json::Value = serde_json::from_str(&write).expect("write json");
        assert_eq!(
            parsed["success"], true,
            "seed write should succeed: {write}"
        );
    }

    let sync = registry
        .try_invoke("Sync", &json!({}), OutputFormat::Markdown)
        .expect("sync invoke");
    assert!(
        sync.contains("Sync complete:"),
        "sync should complete before semantic lookup: {sync}"
    );

    let output = registry
        .try_invoke(
            "FindSimilarConcepts",
            &json!({"conceptName":"authentication","maxResults":5}),
            OutputFormat::Markdown,
        )
        .expect("find similar invoke");

    assert!(
        output.contains("oauth2"),
        "expected related concept missing: {output}"
    );
    assert!(
        output.contains("jwt"),
        "expected related concept missing: {output}"
    );

    Config::reset_overrides();
}

#[test]
fn markdown_reader_sync_should_match_single_pass_extraction_contract() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, _cfg) = isolated_root();

    let registry = ToolRegistry::global();
    let write = registry
        .try_invoke(
            "WriteMemory",
            &json!({
                "title":"Markdown Reader Sync Seed",
                "content":"Alpha [[authentication]] outside code.\n\n```rust\n[[ignored-in-code]]\n```\n\nInline `[[ignored-inline]]` should also be excluded."
            }),
            OutputFormat::Json,
        )
        .expect("write seed");
    let parsed: serde_json::Value = serde_json::from_str(&write).expect("write json");
    assert_eq!(
        parsed["success"], true,
        "seed write should succeed: {write}"
    );

    let sync = registry
        .try_invoke("Sync", &json!({}), OutputFormat::Markdown)
        .expect("sync invoke");
    assert!(
        sync.contains("Sync complete:"),
        "sync should complete before extraction assertions: {sync}"
    );

    let extract = registry
        .try_invoke(
            "ExtractConceptsFromFile",
            &json!({"identifier":"markdown-reader-sync-seed"}),
            OutputFormat::Markdown,
        )
        .expect("extract invoke");

    assert!(
        extract.contains("authentication"),
        "expected non-code wikilink missing: {extract}"
    );
    assert!(
        !extract.contains("ignored-in-code"),
        "code block wikilinks should be excluded: {extract}"
    );
    assert!(
        !extract.contains("ignored-inline"),
        "inline-code wikilinks should be excluded: {extract}"
    );

    Config::reset_overrides();
}
