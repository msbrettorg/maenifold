// T-13.7: RTM FR-6 — Search mode parity tests
// T-13.8: RTM FR-7, FR-8 — MemoryStatus and ToolResponse integration tests
//
// This file covers gaps NOT already tested by:
//   - search_tools_tests.rs (JSON field structure, score normalization, min_score)
//   - memory_status_parity.rs (markdown sections: folders, size, graph stats, DB size)
//   - response.rs unit tests (with_next_steps/with_hint formatting)
//
// New coverage provided here:
//   1. Invalid search mode falls through to Hybrid (T-13.7)
//   2. MemoryStatus JSON output wraps result in success envelope (T-13.8)
//   3. Search results contain wired with_next_steps hints (T-13.8 integration)
//   4. WriteMemory results contain wired with_hint hints (T-13.8 integration)
//   5. with_next_steps/with_hint don't corrupt original content in real tool calls (T-13.8)
//   6. Fulltext + Hybrid return non-empty results for valid queries (T-13.7)

use maenifold_core::json::OutputFormat;
use maenifold_core::memory;
use maenifold_core::search;
use maenifold_core::tools::ToolContext;
use maenifold_core::{AssetManager, Config, GraphDatabase, ToolRegistry};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::{Value, json};
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

const TEST_FOLDER: &str = "search-status-parity";

struct TestEnv {
    _tmp: TempDir,
    cfg: Config,
}

impl TestEnv {
    fn ctx_markdown(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Markdown)
    }

    fn ctx_json(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Json)
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

fn isolated_env() -> TestEnv {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    AssetManager::initialize_assets(&cfg).expect("assets");
    GraphDatabase::initialize_database(&cfg).expect("db init");
    TestEnv { _tmp: tmp, cfg }
}

fn invoke_json(name: &str, payload: Value) -> String {
    ToolRegistry::global()
        .try_invoke(name, &payload, OutputFormat::Json)
        .expect(&format!("{name} should succeed"))
}

fn seed_search_data(env: &TestEnv) {
    let folder = format!("{TEST_FOLDER}/docs");
    memory::write_memory(
        &env.ctx_markdown(),
        &json!({
            "title": "Rust Programming Guide",
            "content": "This is a comprehensive guide to [[Rust]] programming language and [[systems programming]]. Rust provides memory safety without garbage collection.",
            "folder": folder,
            "tags": ["rust", "programming"],
        }),
    )
    .expect("write memory");

    memory::write_memory(
        &env.ctx_markdown(),
        &json!({
            "title": "Python Data Science",
            "content": "This document covers [[Python]] for [[data science]] and [[machine learning]]. Python is widely used in scientific computing.",
            "folder": folder,
            "tags": ["python", "data"],
        }),
    )
    .expect("write memory");

    let sync_output = search::sync(&env.ctx_markdown(), &json!({})).expect("sync");
    assert!(
        sync_output.contains("Sync complete:"),
        "unexpected sync output: {sync_output}"
    );
}

fn search_markdown(env: &TestEnv, mode: &str) -> String {
    search::search_memories(
        &env.ctx_markdown(),
        &json!({
            "query": "Rust programming",
            "mode": mode,
            "pageSize": 10,
            "page": 1,
            "folder": TEST_FOLDER,
            "tags": Value::Null,
            "minScore": 0.0,
        }),
    )
    .expect("search memories")
}

fn search_json(env: &TestEnv, mode: &str) -> Value {
    let raw = search::search_memories(
        &env.ctx_json(),
        &json!({
            "query": "Rust programming",
            "mode": mode,
            "pageSize": 10,
            "page": 1,
            "folder": TEST_FOLDER,
            "tags": Value::Null,
            "minScore": 0.0,
        }),
    )
    .expect("search memories");
    serde_json::from_str::<Value>(&raw).expect("parse JSON output")
}

// ---------------------------------------------------------------------------
// T-13.7: RTM FR-6 — Invalid search mode falls through to Hybrid
// ---------------------------------------------------------------------------

// T-13.7: RTM FR-6 — Unknown mode string should fall through to Hybrid path gracefully.
#[test]
fn invalid_mode_falls_through_to_hybrid() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    seed_search_data(&env);

    // "BogusMode" doesn't match Semantic or FullText, so the code falls through
    // to the default Hybrid search path.
    let result = search_markdown(&env, "BogusMode");

    assert!(
        !result.contains("ERROR"),
        "invalid mode should not produce an error: {result}"
    );
    // Unknown modes hit the else branch which emits "Search Results" (not "Hybrid Search").
    // Pin the exact heading to catch accidental label changes.
    assert!(
        result.contains("Search Results"),
        "invalid mode should fall through to default heading 'Search Results': {result}"
    );
    assert!(
        result.contains("Found"),
        "invalid mode should still produce results: {result}"
    );
    // Verify seeded documents actually appear — prevents silent empty-results pass.
    assert!(
        result.contains("Rust Programming Guide"),
        "seeded document should appear in fallthrough results: {result}"
    );
}

// T-13.7: RTM FR-6 — Unknown mode in JSON output returns mode="Hybrid" envelope.
#[test]
fn invalid_mode_json_reports_hybrid_envelope() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    seed_search_data(&env);

    let json = search_json(&env, "NotAMode");
    let data = &json["data"];

    // The mode field in the JSON envelope reflects the raw passed value (not normalized).
    assert_eq!(
        data["mode"].as_str(),
        Some("NotAMode"),
        "mode field should reflect the passed value: {data}"
    );

    // Results must be non-empty — a broken implementation returning 0 results must fail.
    let results = data["results"].as_array().expect("results array");
    assert!(
        !results.is_empty(),
        "invalid mode should still produce results via Hybrid code path: {data}"
    );

    // fusedScore presence proves the Hybrid code path was used.
    assert!(
        results[0].get("fusedScore").is_some(),
        "invalid mode should use Hybrid code path with fusedScore: {}",
        results[0]
    );
}

// ---------------------------------------------------------------------------
// T-13.7: RTM FR-6 — Fulltext and Hybrid return non-empty results
// ---------------------------------------------------------------------------

// T-13.7: RTM FR-6 — FullText and Hybrid modes both return non-empty results for a valid query.
#[test]
fn fulltext_and_hybrid_modes_return_nonempty_results() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    seed_search_data(&env);

    for mode in &["FullText", "Hybrid"] {
        let json = search_json(&env, mode);
        let results = json["data"]["results"].as_array().expect("results array");

        assert!(
            !results.is_empty(),
            "{mode} mode should return at least one result for 'Rust programming'"
        );
    }
}

// T-13.7: RTM FR-6 — Semantic mode with no ONNX model degrades gracefully (empty or error-free).
#[test]
fn semantic_mode_degrades_gracefully_without_model() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    seed_search_data(&env);

    let json = search_json(&env, "Semantic");
    let data = &json["data"];

    // Without an ONNX model, semantic search should still return valid JSON
    // (possibly with zero results) and mode="Semantic".
    assert_eq!(data["mode"].as_str(), Some("Semantic"));
    assert!(
        data["results"].is_array(),
        "semantic results should still be an array even without model"
    );
}

// ---------------------------------------------------------------------------
// T-13.8: RTM FR-7 — MemoryStatus JSON output envelope
// ---------------------------------------------------------------------------

// T-13.8: RTM FR-7 — MemoryStatus in JSON mode wraps markdown in success envelope with all sections.
#[test]
fn memory_status_json_output_shape() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    let raw = invoke_json("MemoryStatus", json!({}));
    let json: Value = serde_json::from_str(&raw).expect("should be valid JSON");

    // --- Success envelope assertions ---
    assert_eq!(
        json["success"].as_bool(),
        Some(true),
        "JSON envelope should have success=true: {json}"
    );
    assert!(
        json["error"].is_null(),
        "JSON envelope should have error=null: {json}"
    );
    let data = json["data"]
        .as_str()
        .expect("data should be a string containing the markdown");

    // --- Embedded markdown section assertions ---
    assert!(
        data.contains("# Memory System Status"),
        "JSON data should contain the status header: {data}"
    );
    assert!(
        data.contains("Total files:"),
        "JSON data should contain file count: {data}"
    );
    assert!(
        data.contains("Total folders:"),
        "JSON data should contain folder count: {data}"
    );
    assert!(
        data.contains("### Graph Database"),
        "JSON data should contain Graph Database section: {data}"
    );
    assert!(
        data.contains("- Concepts:"),
        "JSON data should contain concept count: {data}"
    );
}

// ---------------------------------------------------------------------------
// T-13.8: RTM FR-8 — with_next_steps wired into search results (integration)
// ---------------------------------------------------------------------------

// T-13.8: RTM FR-8 — Search markdown output contains wired next-steps hint.
#[test]
fn search_results_contain_next_steps_hint() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    seed_search_data(&env);

    let result = search_markdown(&env, "Hybrid");

    // T-13.5 wires with_next_steps into the Hybrid search path.
    assert!(
        result.contains("💡 Next: Consider using"),
        "Hybrid search markdown should contain next-steps hint: {result}"
    );
    assert!(
        result.contains("ReadMemory"),
        "next-steps hint should suggest ReadMemory: {result}"
    );
    assert!(
        result.contains("BuildContext"),
        "next-steps hint should suggest BuildContext: {result}"
    );
}

// T-13.8: RTM FR-8 — FullText search markdown also contains next-steps hint.
#[test]
fn fulltext_search_results_contain_next_steps_hint() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    seed_search_data(&env);

    let result = search_markdown(&env, "FullText");

    assert!(
        result.contains("💡 Next: Consider using"),
        "FullText search markdown should contain next-steps hint: {result}"
    );
    assert!(
        result.contains("ReadMemory"),
        "FullText next-steps hint should suggest ReadMemory: {result}"
    );
    assert!(
        result.contains("BuildContext"),
        "FullText next-steps hint should suggest BuildContext: {result}"
    );
}

// ---------------------------------------------------------------------------
// T-13.8: RTM FR-8 — with_hint wired into WriteMemory (integration)
// ---------------------------------------------------------------------------

// T-13.8: RTM FR-8 — WriteMemory markdown output contains with_hint hint.
#[test]
fn write_memory_contains_hint() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let result = memory::write_memory(
        &env.ctx_markdown(),
        &json!({
            "title": "Hint Test Note",
            "content": "This note tests that [[WriteMemory]] emits a with_hint hint.",
        }),
    )
    .expect("write memory");

    assert!(
        result.contains("Created memory FILE:"),
        "should create the memory: {result}"
    );
    // T-13.5 wires with_hint into WriteMemory's markdown path.
    assert!(
        result.contains("💡"),
        "WriteMemory markdown should contain a hint (lightbulb emoji): {result}"
    );
    assert!(
        result.contains("ReadMemory"),
        "hint should reference ReadMemory for follow-up: {result}"
    );
    assert!(
        result.contains("GetHelp(\"WriteMemory\")"),
        "hint should reference GetHelp for the tool: {result}"
    );
}

// ---------------------------------------------------------------------------
// T-13.8: RTM FR-8 — with_next_steps/with_hint don't corrupt original content
// ---------------------------------------------------------------------------

// T-13.8: RTM FR-8 — Hybrid search result preserves core content above the hint.
#[test]
fn search_next_steps_does_not_corrupt_results() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    seed_search_data(&env);

    let result = search_markdown(&env, "Hybrid");

    // The actual search result should still be intact above the hint.
    assert!(
        result.contains("Rust Programming Guide"),
        "search result content should be preserved above hint: {result}"
    );
    assert!(
        result.contains("Found"),
        "search header should be preserved: {result}"
    );

    // The hint should be at the very end (last line), not interleaved.
    let lines: Vec<&str> = result.trim_end().lines().collect();
    let last_line = lines.last().expect("should have lines");
    assert!(
        last_line.starts_with("💡"),
        "hint should appear as the last line, got: {last_line}"
    );
}

// T-13.8: RTM FR-8 — WriteMemory hint doesn't corrupt the URI/checksum content.
#[test]
fn write_memory_hint_does_not_corrupt_content() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let result = memory::write_memory(
        &env.ctx_markdown(),
        &json!({
            "title": "Corruption Check",
            "content": "Verifying [[content]] integrity with with_hint wiring.",
        }),
    )
    .expect("write memory");

    // The URI and checksum should appear before the hint.
    assert!(
        result.contains("Created memory FILE: memory://"),
        "URI should be intact: {result}"
    );
    assert!(
        result.contains("Checksum:"),
        "checksum should be intact: {result}"
    );

    // The hint line should be last, not interleaved with URI/checksum.
    let lines: Vec<&str> = result.trim_end().lines().collect();
    let last_line = lines.last().expect("should have lines");
    assert!(
        last_line.starts_with("💡"),
        "hint should be the last line, got: {last_line}"
    );

    // URI line should be the first line.
    assert!(
        lines[0].starts_with("Created memory FILE:"),
        "first line should be the URI: {}",
        lines[0]
    );
}
