use maenifold_core::{
    AssetManager, Config, GraphDatabase, json::OutputFormat, memory, search, tools::ToolContext,
};
use serde_json::{Value, json};
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::{
    LazyLock, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use tempfile::TempDir;

const TEST_FOLDER: &str = "cli-json-perf-tests";
const QUERY: &str = "perf-regression-keyword";

static TEST_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
static MEASURE_ALLOCATIONS: AtomicBool = AtomicBool::new(false);
static ALLOCATED_BYTES: AtomicUsize = AtomicUsize::new(0);

struct CountingAllocator;

#[global_allocator]
static GLOBAL_ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Safety: delegating to the process-global system allocator with the provided layout.
        let ptr = unsafe { System.alloc(layout) };
        if MEASURE_ALLOCATIONS.load(Ordering::Relaxed) && !ptr.is_null() {
            ALLOCATED_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // Safety: delegating to the process-global system allocator with the provided layout.
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if MEASURE_ALLOCATIONS.load(Ordering::Relaxed) && !ptr.is_null() {
            ALLOCATED_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // Safety: delegating to the process-global system allocator using the original layout.
        unsafe { System.dealloc(ptr, layout) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // Safety: delegating to the process-global system allocator using the original layout.
        let ptr = unsafe { System.realloc(ptr, layout, new_size) };
        if MEASURE_ALLOCATIONS.load(Ordering::Relaxed) && !ptr.is_null() {
            ALLOCATED_BYTES.fetch_add(new_size, Ordering::Relaxed);
        }
        ptr
    }
}

struct TestEnv {
    _tmp: TempDir,
    config: Config,
}

impl TestEnv {
    fn new() -> Self {
        let tmp = TempDir::new().expect("tmp");
        Config::override_root(tmp.path());
        Config::set_database_path(tmp.path().join("memory.db"));

        let config = Config::current();
        config.ensure_directories().expect("dirs");
        AssetManager::initialize_assets(&config).expect("assets");
        GraphDatabase::initialize_database(&config).expect("db init");

        let env = Self { _tmp: tmp, config };
        env.seed_dataset();
        env
    }

    fn context(&self, output_format: OutputFormat) -> ToolContext {
        ToolContext::new(self.config.clone(), output_format)
    }

    fn seed_dataset(&self) {
        // T-0.1 / T-CLI-JSON-001.6a: RTM NFR-3 - seed a deterministic fixture set before comparing markdown vs JSON output behavior.
        let context = self.context(OutputFormat::Markdown);
        for index in 0..30 {
            let title = format!("Perf Regression Seed {index:02}");
            let body = format!(
                "Seed file {index:02} for [[cli-json-output]] perf regression.\n\nContains the fixed query token: {QUERY}\n\nRelated concepts: [[search]] [[json]] [[markdown]] [[performance-testing]]."
            );
            let output = memory::write_memory(
                &context,
                &json!({
                    "title": title,
                    "content": body,
                    "folder": TEST_FOLDER,
                }),
            )
            .expect("write memory");
            assert!(
                output.starts_with("Created memory FILE:"),
                "unexpected write output: {output}"
            );
        }

        let sync_output = search::sync(&context, &json!({})).expect("sync");
        assert!(
            sync_output.contains("Sync complete:"),
            "unexpected sync output: {sync_output}"
        );
    }

    fn run_search(&self, output_format: OutputFormat) -> String {
        // T-2.3 / T-CLI-JSON-001.6a: RTM FR-4 - exercise the same SearchMemories full-text payload in both CLI output formats.
        search::search_memories(
            &self.context(output_format),
            &json!({
                "query": QUERY,
                "mode": "FullText",
                "pageSize": 10,
                "page": 1,
                "folder": TEST_FOLDER,
            }),
        )
        .expect("search memories")
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

fn measure_allocated_bytes<F>(action: F, iterations: usize) -> usize
where
    F: Fn() -> String,
{
    ALLOCATED_BYTES.store(0, Ordering::Relaxed);
    MEASURE_ALLOCATIONS.store(true, Ordering::SeqCst);

    for _ in 0..iterations {
        black_box(action().len());
    }

    MEASURE_ALLOCATIONS.store(false, Ordering::SeqCst);
    ALLOCATED_BYTES.load(Ordering::Relaxed)
}

#[test]
fn search_memories_fulltext_json_mode_does_not_allocate_significantly_more_than_markdown() {
    // T-0.1 / T-CLI-JSON-001.6a: RTM NFR-3 - guard against JSON-mode double work using deterministic allocation deltas on the same dataset.
    let _guard = TEST_MUTEX.lock().expect("mutex");
    let env = TestEnv::new();

    black_box(env.run_search(OutputFormat::Markdown).len());
    black_box(env.run_search(OutputFormat::Json).len());

    const ITERATIONS: usize = 12;
    let markdown_allocated =
        measure_allocated_bytes(|| env.run_search(OutputFormat::Markdown), ITERATIONS);
    let json_allocated = measure_allocated_bytes(|| env.run_search(OutputFormat::Json), ITERATIONS);

    let slack = usize::max(4096, markdown_allocated / 10);
    assert!(
        json_allocated <= markdown_allocated + slack,
        "JSON mode allocated {json_allocated} bytes vs markdown {markdown_allocated} bytes (slack {slack})."
    );
}

#[test]
fn search_memories_fulltext_json_mode_output_size_ratio_is_bounded() {
    // T-0.1 / T-CLI-JSON-001.6a: RTM NFR-3 - bound JSON serialization overhead and reject markdown-shaped output inside the JSON contract.
    let _guard = TEST_MUTEX.lock().expect("mutex");
    let env = TestEnv::new();

    let markdown = env.run_search(OutputFormat::Markdown);
    let json = env.run_search(OutputFormat::Json);

    assert!(!markdown.is_empty(), "markdown output should not be empty");
    assert!(!json.is_empty(), "json output should not be empty");
    assert!(
        markdown.contains("# Full-Text Search"),
        "markdown output should preserve the full-text heading: {markdown}"
    );

    let parsed: Value = serde_json::from_str(&json).expect("valid json output");
    assert_eq!(
        parsed["success"], true,
        "unexpected JSON envelope: {parsed}"
    );
    assert!(
        parsed["data"]["results"].is_array(),
        "search JSON should expose structured results: {parsed}"
    );

    assert!(
        !json.contains("Full-Text Search"),
        "JSON output should not embed markdown headings: {json}"
    );
    assert!(
        !json.contains("💡 Next:"),
        "JSON output should not embed markdown follow-up prompts: {json}"
    );

    let ratio = json.len() as f64 / markdown.len() as f64;
    assert!(
        ratio < 2.75,
        "Unexpected JSON/markdown output size ratio: {ratio:.3}"
    );
}
