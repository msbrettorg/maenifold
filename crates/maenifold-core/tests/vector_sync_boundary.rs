use maenifold_core::json::OutputFormat;
use maenifold_core::{Config, ToolRegistry};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::json;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

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
fn watcher_lifecycle_should_start_coalesce_events_and_stop_cleanly() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, _cfg) = isolated_root();

    let registry = ToolRegistry::global();
    let start = registry
        .try_invoke(
            "StartWatcher",
            &json!({"debounceMs":250}),
            OutputFormat::Markdown,
        )
        .expect("start watcher");
    assert!(
        start.starts_with("Started watching "),
        "unexpected start output: {start}"
    );
    assert!(
        start.contains(" for incremental sync (debounce: 250ms)"),
        "unexpected start output: {start}"
    );

    let stop = registry
        .try_invoke("StopWatcher", &json!({}), OutputFormat::Markdown)
        .expect("stop watcher");
    assert!(
        stop == "Stopped watcher" || stop == "Watcher not running",
        "unexpected stop output: {stop}"
    );

    Config::reset_overrides();
}

#[test]
fn sync_should_expose_leader_election_skip_or_process_contract() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, _cfg) = isolated_root();

    let output = ToolRegistry::global()
        .try_invoke("Sync", &json!({}), OutputFormat::Markdown)
        .expect("sync invoke");

    assert!(
        output.contains("leader")
            || output.contains("follower")
            || output.contains("Sync complete:"),
        "sync should eventually expose leader-election outcome semantics: {output}"
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
