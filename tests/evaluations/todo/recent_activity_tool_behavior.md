# Test Evaluation: `recent_activity_tool_behavior.rs`

> Evaluated under **Gray (Skeptical Inquiry)** — question assumptions, demand evidence, challenge
> conventional wisdom.
>
> Traceability: **T-2.3** / **FR-4** (RTM-020–030)

---

## Summary

`recent_activity_tool_behavior.rs` contains exactly **one `#[test]` function**:

```
recent_activity_filters_timespan_include_content_workflow_and_sequential
```

This single function is a **monolithic mega-test** that packs nine distinct behavioral scenarios
into a linear sequence with no isolation between them. It exercises the full seeding → Sync →
RecentActivity pipeline, but every behavior it asserts is already covered — usually more precisely
— by the existing split test files:

| File | Tests |
|---|---|
| `recent_activity_boundary_queries.rs` | 4 focused query-oriented tests |
| `recent_activity_boundary_rendering.rs` | 4 focused rendering / empty-state tests |
| `recent_activity_pipeline_collect.rs` | 7 unit tests of the collect layer |
| `recent_activity_pipeline_format.rs` | 8 format-layer unit tests |
| `recent_activity_pipeline_helpers.rs` | 14 pure-function unit tests |
| `recent_activity_pipeline_tool.rs` | 7 tool-layer integration tests |

The file contributes **noise, not signal**. Every failure it can produce will be produced first —
and with better locality — by one of the 44 tests above.

---

## Verdicts

### Behavior Block Inventory

The single test function contains nine sequential behavioral segments:

| # | Lines | Behavior | Verdict |
|---|-------|----------|---------|
| B1 | 49–57 | Sync runs successfully, DB exists | **REMOVE** — scaffolding masquerading as assertion |
| B2 | 61–77 | `all` filter returns sequential + workflow + memory items | **REMOVE** — fully duplicated |
| B3 | 79–94 | `thinking` filter includes sequential + workflow; excludes memory | **REMOVE** — fully duplicated |
| B4 | 96–108 | `memory` filter includes memory; excludes sequential + workflow | **REMOVE** — fully duplicated |
| B5 | 110–129 | `assumptions` filter returns chat as memory type | **REMOVE** — duplicated AND weaker assertion |
| B6 | 131–150 | `includeContent: true` surfaces `First H2:` / `First H2 Content:` fields | **REMOVE** — duplicated AND weaker assertion |
| B7 | 152–167 | `timespan` excludes old entry; keeps recent sequential + workflow | **REMOVE** — fully duplicated |
| B8 | 169–179 | Negative timespan returns `InvalidArgument` error | **REMOVE** — fully duplicated |
| B9 | 181–186 | `limit: 1` caps results to one rendered item | **REWRITE** — see below |

### Test Function Verdict

| Test | Load-Bearing? | Unique? | Assertions Correct? | Name Accurate? | Parameterizable? | Verdict |
|------|:---:|:---:|:---:|:---:|:---:|---------|
| `recent_activity_filters_timespan_include_content_workflow_and_sequential` | Marginal | No | Partially — see findings | No | Yes (filter variants) | **REWRITE** |

---

## Findings

### F-1 · CRITICAL: Nine behaviors, one test — attribution impossible on failure

**What it does**: All nine behavior blocks share a single test function name, a single mutex
guard, and a single isolated environment. When block B5 (assumptions filter) fails, the test
runner reports:

```
thread 'recent_activity_filters_timespan_include_content_workflow_and_sequential' panicked
```

There is no way to know from that output which of the nine behaviors failed without reading the
assertion message. Blocks B3 through B9 will never run when B2 panics.

**Evidence that coverage already exists**: every behavior has a named twin in the split files.

---

### F-2 · HIGH: The OR assertion in B5 conceals undefined behavior

```rust
// Lines 119–120
assert!(
    chat_out.contains("Title: chat-1") || chat_out.contains("Title: Chat Log"),
    "assumptions: expected chat title: {chat_out}"
);
```

This assertion passes if *either* the file-path stem **or** the H1 heading appears in the
output. It would also pass if the tool produced the stem when it should produce the heading, or
vice versa — a silent regression. The boundary test `recent_activity_filter_variants_select_expected_path_families`
pins to `Title: Assumption Log` (a known value); this test does not.

**Gray verdict**: an OR in an assertion is a question mark about what the expected behavior
actually is. Pin it or remove it.

---

### F-3 · MEDIUM: B6 (includeContent) checks field *names*, not field *values*

```rust
// Lines 135–142
assert!(include_out.contains("First H2:"), ...);
assert!(include_out.contains("First H2 Content:"), ...);
assert!(include_out.contains("Thought 1/2"), ...);
assert!(include_out.contains("Step 1"), ...);
```

The first two assertions verify only that the label keys exist in the output; they do not
verify the values after the colon. A broken formatter that emits `First H2: ???` would pass.
`recent_activity_include_content_includes_h2_heading_and_content` checks
`First H2: Thought 1` and `First H2 Content: First content block` — full key-value pairs.

---

### F-4 · MEDIUM: B8 uses a different invocation path than B1–B7 with no explanation

Blocks B1–B7 and B9 all call `invoke_tool(...)` (which wraps `ToolRegistry::global().try_invoke`
and unwraps errors). Block B8 calls `ToolRegistry::global().try_invoke` directly:

```rust
// Lines 169–173
let bad = ToolRegistry::global().try_invoke(
    "RecentActivity",
    &json!({ "timespan": "-1.00:00:00" }),
    OutputFormat::Markdown,
);
```

The inconsistency implies this was an afterthought. The pipeline test
`recent_activity_negative_timespan_returns_error` calls `tools::recent_activity` directly and
matches `MaenifoldError::InvalidArgument` — a more precise assertion.

---

### F-5 · LOW: B1 (Sync) asserts on Sync's output, not RecentActivity's

```rust
// Lines 50–53
assert!(
    sync_output.contains("Sync"),
    "Sync should run successfully: {sync_output}"
);
```

The string "Sync" will match any Sync output ever produced. This assertion has near-zero
discriminative power. The test is not named after Sync behavior; it should not be asserting on
Sync output at all — it should just invoke Sync as setup and proceed.

---

### F-6 · LOW: Test name is an incomplete enumeration, not a concept

`recent_activity_filters_timespan_include_content_workflow_and_sequential`

This name:
- Lists some (not all) behaviors: mentions `filters`, `timespan`, `include_content`, `workflow`,
  `sequential`
- Omits: `memory` filter, `assumptions` filter, negative-timespan validation, `limit`
- Reads as a laundry list rather than a concept ("full pipeline smoke test")

A name that accurately describes a 9-behavior test would be 30+ words. That is a strong
signal the test should be multiple tests.

---

### F-7 · INFORMATIONAL: B9 (limit=1) has marginal uniqueness

The existing limit tests all use `limit: 2`. Testing `limit: 1` exercises the boundary between
"at least one result" and "empty result". This is the only non-zero unique contribution of the
test. If the test is rewritten, a focused `limit_one_returns_exactly_one_item` test should be
extracted rather than discarded.

---

### F-8 · INFORMATIONAL: The test uses file-seeding + Sync, not direct DB injection

The boundary tests in `recent_activity_boundary_queries.rs` inject rows directly into
`file_content` via `seed_activity()`. This test seeds actual files and invokes `Sync` first.
That seam is the one thing this test exercises that the boundary tests do not.

However, the pipeline tool tests (`recent_activity_pipeline_tool.rs`) also use
file-seeding + Sync. The only partial uniqueness is that this test seeds all four file types
(memory, sequential, workflow, chat) simultaneously through the same Sync run, while the
pipeline_tool tests tend to seed subsets. This is marginal unique value.

---

## Recommendations

### 1. REWRITE as a narrow integration smoke test (priority: HIGH)

Strip the function down to its one genuinely unique contribution — verifying that all four file
types survive the full seed → Sync → RecentActivity pipeline together:

```rust
#[test]
fn recent_activity_all_file_types_survive_full_sync_pipeline() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_memory_file(&env.cfg, "article-alpha.md", "# Alpha\n\n## S\nBody\n");
    seed_sequential_file(&env.cfg, "session-1", "## Thought 1/1\nContent");
    seed_workflow_file(&env.cfg, "workflow-1", "## Step 1\nContent");
    seed_chat_file(&env.cfg, "chat-1.md", "# Chat\n\n## Msg\nHello\n");

    invoke_tool("Sync", json!({}));

    let out = invoke_tool("RecentActivity", json!({ "limit": 10 }));
    assert!(out.contains("(sequential)"), "sequential missing: {out}");
    assert!(out.contains("(workflow)"),   "workflow missing: {out}");
    assert!(out.contains("(memory)"),     "memory missing: {out}");
}
```

That is the entire load-bearing core of this file. It is one test.

### 2. EXTRACT limit=1 edge case (priority: MEDIUM)

Add a dedicated test either here or in `recent_activity_boundary_queries.rs`:

```rust
#[test]
fn recent_activity_limit_one_returns_exactly_one_item() {
    // seed two+ items; assert rendered_item_count == 1
}
```

### 3. REMOVE all redundant behavior blocks (priority: HIGH)

B2–B8 are fully covered by the split test suite with stronger assertions. Keeping them
accumulates maintenance debt with zero additional protection.

### 4. PIN the assumptions title assertion (priority: MEDIUM)

If the `assumptions` filter test is retained anywhere, replace the OR with a specific expected
value. Decide whether the tool renders the file-path stem or the H1 heading, document it in
the RTM (RTM-023), and assert exactly that.

### 5. Consider: does `recent_activity_tool_behavior.rs` justify existence?

After the recommended rewrite, this file contains one smoke test. Evaluate whether that test
belongs in `recent_activity_pipeline_tool.rs` (which already has 7 integration tests using the
same seeding pattern) and delete this file entirely to reduce directory surface area.

---

## Load-Bearing Score: 1/9

One of the nine behavior blocks carries marginal unique value (B8 — limit=1 edge case, and the
combined all-four-file-types-through-sync integration). Eight blocks are pure duplication of
the existing 44-test split suite, with weaker assertions in two cases (B5, B6). The file as
written provides **negative value** on the margin: it adds noise to failure reports, masks
regressions behind OR conditions, and gives maintainers false confidence that covered behaviors
are validated here to a higher standard than they are.
