---
id: contracts
status: done
pipeline: tdd
---

# Test Evaluation: contracts.rs

> **Traceability**: NFR-2 (test coverage), NFR-3 (no regressions)  
> **Reviewer color**: 🔘 Gray — Skeptical Inquiry  
> **Method**: Read every `#[test]`. Cross-reference against overlapping test files. Query source
> schema DDL. Verify assertions are correct and complete. Ask: _what is this actually proving?_

---

## Summary

`contracts.rs` is a 121-line file containing **5 tests** positioned as high-level integration
smoke tests for the four public subsystems: `Config`, `ToolRegistry`, `GraphDatabase`, and
`AssetManager`. The intent is sound — a "contracts" file should establish that the core
integration seams hold together. In practice, this file is uneven: two tests do their job well,
one is a diluted duplicate of a more complete sibling, one tests a genuine contract with a
weak assertion set, and one has a factual correctness gap in its table coverage.

**Key gray observations before the verdicts:**

1. **`isolated_config()` does not initialise the database.** Only one test (`graph_database_creates_shared_schema_tables`) calls `GraphDatabase::initialize_database()` explicitly. The other four tests that call `isolated_config()` correctly do not need the DB — but this asymmetry is invisible to a reader who assumes the helper is "full" setup.

2. **`TEST_MUTEX` is a file-local static.** `contracts.rs` and `tool_registry_tests.rs` each define their own `Lazy<Mutex<()>>`. Because `Config::override_root()` modifies process-global state, two tests from _different_ files running concurrently can race even though both hold "their" mutex. This is a systemic design smell across the test suite, not unique to contracts.rs.

3. **The file has no `T-*` traceability comments.** Every test in `tool_registry_tests.rs` and `db_init_pragma_tests.rs` is annotated with `// T-X.Y: RTM FR-Z`. The five tests here have no such anchors. If the suite grows, these will become orphaned.

4. **`graph_database_creates_shared_schema_tables` misses 2/7 core tables.** The DDL in `schema.rs` creates `file_content`, `file_search`, `concepts`, `concept_mentions`, `concept_graph`, `concept_communities`, `sync_leader_lease`. The test checks five, silently skipping `concepts` and `concept_mentions` — the two most referenced tables in the codebase.

5. **`tool_registry_is_case_insensitive_and_exposes_contract_names` is subsumed.** `tool_registry_tests.rs` contains `new_registry_registers_all_expected_tool_names` (checks all 28 tools by name against a `BTreeSet`) and `try_invoke_case_insensitive_lookup_always_resolves` (tests four casing variants including full-uppercase). The contracts version spot-checks three names and one canonical-names query — strictly weaker.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Assertions Correct? | Name Clear? | Parameterisable? | Verdict |
|---|-----------|:-------------:|:-------:|:-------------------:|:-----------:|:----------------:|---------|
| 1 | `config_honors_override_paths` | Moderate | ✅ Yes — no dedicated config test file | ⚠️ Partial — checks suffix strings, not resolved paths; no check that dirs exist | ✅ Yes | ❌ No | **REWRITE** |
| 2 | `tool_registry_is_case_insensitive_and_exposes_contract_names` | ❌ No | ❌ No — fully subsumed by `tool_registry_tests.rs` | ⚠️ Spot-checks 3/28 tools | ⚠️ Over-claims ("exposes contract names" but only checks 1) | ❌ No | **REMOVE** |
| 3 | `graph_database_creates_shared_schema_tables` | ✅ Yes — only test querying `sqlite_master` for table existence | ✅ Yes | ❌ No — misses `concepts` and `concept_mentions` (2/7 tables from `schema.rs`) | ✅ Yes | ✅ Yes | **REWRITE** |
| 4 | `assets_bootstrap_and_catalog_are_available` | ✅ Yes | ✅ Yes — unique catalog query + URI routing coverage | ⚠️ Partial — `!is_empty()` is weak; hardcoded `"deductive-reasoning"` URI is brittle | ✅ Yes | ❌ No | **REWRITE** |
| 5 | `catalog_uses_current_config_assets_path_by_default` | ✅ Yes | ✅ Yes | ✅ Strong — sentinel-file approach directly proves path binding | ✅ Yes | ❌ No | **KEEP** |

**Load-Bearing Score: 4/5**

---

## Detailed Findings

---

### Test 1 — `config_honors_override_paths`

**Verdict: REWRITE**

**What it does:** Calls `isolated_config()`, then asserts:
- `cfg.memory_path.ends_with("memory")`
- `cfg.assets_path.ends_with("assets")`
- `cfg.database_path.file_name() == Some("memory.db")`

**Why the assertions are weak:**  
`ends_with("memory")` passes if the path is
`/some/path/memory` but also if it ends with any segment containing "memory" as a
suffix (e.g., `/other-memory`). The correct assertion is to check the full resolved path
segment, or use `Path::ends_with` with a `Path` not a string (Rust's `ends_with(&str)` on
`Path` does full-component matching, but the string literal `"memory"` will match any
component named exactly `"memory"` — which is actually fine in Rust's stdlib, but the
intent is unclear to a reader).

More importantly: **the test does not verify that the directories were created on disk.**
`cfg.ensure_directories()` is called but not validated. If `ensure_directories()` silently
fails, subsequent tools that write files will fail with confusing errors, not here.

**Missing assertions:**
```rust
assert!(cfg.memory_path.is_dir(), "memory dir should exist");
assert!(cfg.assets_path.is_dir(), "assets dir should exist");
assert!(cfg.database_path.parent().unwrap().is_dir(), "db parent dir should exist");
```

**Traceability gap:** No `T-*` or `RTM` comment. Config infrastructure is foundational to
FR-22 (`ToolContext` path migration) — this test should be anchored.

---

### Test 2 — `tool_registry_is_case_insensitive_and_exposes_contract_names`

**Verdict: REMOVE**

**What it does:** Asserts `registry.contains("WriteMemory")`, `registry.contains("writememory")`,
`registry.contains("LISTASSETS")`, and that `registry.canonical_names()` includes
`"ReadMcpResource"`.

**Why it's redundant:**  
`tool_registry_tests.rs` already provides:

| Existing test | Superset of this test? |
|---|---|
| `new_registry_registers_all_expected_tool_names` | ✅ Checks all 28 tools by name. Includes WriteMemory, LISTASSETS, ReadMcpResource. |
| `try_invoke_case_insensitive_lookup_always_resolves` | ✅ Tests four casing variants: `GETCONFIG`, `GetConfig`, `getconfig`, `gEtCoNfIg`. |

The contracts version adds no new coverage. It would not catch a regression that the
`tool_registry_tests.rs` wouldn't catch first — and with less diagnostic output (it only
mentions 3 tools in its failure messages; the other file prints a full `BTreeSet` diff).

**Acquisition cost of keeping it:** Every time the tool list changes, engineers must update
both files. The contracts version has no `T-*` anchoring to explain _why_ it exists alongside
the more complete file.

---

### Test 3 — `graph_database_creates_shared_schema_tables`

**Verdict: REWRITE**

**What it does:** Calls `GraphDatabase::initialize_database()`, opens a raw connection, queries
`sqlite_master` for table and view names, then asserts presence of:
`file_content`, `file_search`, `concept_graph`, `concept_communities`, `sync_leader_lease`.

**The factual correctness gap:**  
`schema.rs` creates **seven** objects:
1. `file_content` — ✅ checked
2. `file_search` (FTS5 virtual) — ✅ checked
3. `concepts` — **❌ NOT checked**
4. `concept_mentions` — **❌ NOT checked**
5. `concept_graph` — ✅ checked
6. `concept_communities` — ✅ checked
7. `sync_leader_lease` — ✅ checked

`concepts` and `concept_mentions` are the two tables with the most downstream referential
dependencies (FK cascades in `db_init_pragma_tests.rs` use them, all graph traversal queries
read them). Their absence from a "creates schema tables" test is a false sense of coverage.

If the `concepts` CREATE TABLE statement were accidentally deleted from `schema.rs`, this
test would still pass. That is a load-bearing blind spot.

**Additionally:** The query `WHERE type IN ('table','view')` will not return virtual FTS5
tables in SQLite unless they also have type `'table'` (they do — SQLite stores FTS5 as
shadow tables + a virtual table entry, but the virtual table entry has `type='table'` in
practice). This is not a bug, but it creates an implicit assumption worth documenting.

**Parameterisable fix:**
```rust
for expected in [
    "file_content",
    "file_search",
    "concepts",           // ← missing
    "concept_mentions",   // ← missing
    "concept_graph",
    "concept_communities",
    "sync_leader_lease",
] { ... }
```

---

### Test 4 — `assets_bootstrap_and_catalog_are_available`

**Verdict: REWRITE**

**What it does:** Calls `AssetManager::new(&cfg).catalog()`, then asserts:
- `!catalog.single_agent_workflows.is_empty()`
- `!catalog.roles.is_empty()`
- `!catalog.colors.is_empty()`
- `!catalog.perspectives.is_empty()`

Then reads `"asset://workflows/deductive-reasoning"` and checks `id == "deductive-reasoning"`.

**Why the `!is_empty()` assertions are weak:**  
These pass with a single item in each collection. They would not catch a regression where,
say, 14 of 16 workflows failed to load but one survived. The meaningful contract is that
the count matches the packaged asset count. If the exact count is too brittle, a minimum
floor (`> 3`, `> 1`) is still stronger than `!= 0`.

**The hardcoded URI is a brittle integration pin:**  
`"asset://workflows/deductive-reasoning"` will cause a spurious failure if that workflow
file is ever renamed or reorganised, even though the routing mechanism itself is fine.
A better approach: read any URI from the catalog itself (use the first workflow's `id`
field to construct the URI), proving the routing is correct for the actual catalog
contents rather than a hardcoded name.

**What IS good here:**  
This is the **only test that exercises `read_resource()` URI routing.** That makes it
genuinely load-bearing despite the weak assertions. It also uniquely verifies all four
asset type collections are populated, which `asset_manager_initialize.rs` does not.
This coverage is worth keeping — the test just needs sharper assertions.

---

### Test 5 — `catalog_uses_current_config_assets_path_by_default`

**Verdict: KEEP**

**What it does:** Creates a sentinel JSON file at `cfg.assets_path/roles/{sentinel_id}.json`,
calls `AssetManager::new(&cfg).catalog()`, and asserts the sentinel appears in `catalog.roles`.

**Why this is the strongest test in the file:**  
The sentinel-file technique directly proves that `AssetManager::new()` reads from
`cfg.assets_path` at runtime — not from a hardcoded compile-time path. This would catch
a regression where someone replaced `cfg.assets_path` with a `std::env::var("ASSETS_PATH")`
lookup that resolves to a different directory.

The unique suffix ensures no cross-test contamination. The `Config::reset_overrides()` is
properly called at the end. The test name precisely describes the contract.

**Minor improvement opportunity:**  
The sentinel content has a trailing space before the closing brace
(`"prompt":"sentinel"}} ` — note the two `}}` which is a format string literal artifact).
This is cosmetically odd but does not affect correctness. Trim it for cleanliness.

---

## Recommendations

Listed by priority:

### P0 — Correctness: Fix the missing tables in Test 3

**Action:** Add `"concepts"` and `"concept_mentions"` to the `for expected in [...]` loop.
These are the most-referenced tables in the schema. Their omission is a factual defect in
the contract, not a style issue. One-line fix.

### P1 — Remove Test 2 (redundant)

**Action:** Delete `tool_registry_is_case_insensitive_and_exposes_contract_names`.
`tool_registry_tests.rs` is strictly more complete. Keeping both requires double-maintenance
with zero coverage benefit. If a "contracts file should include one registry smoke test"
policy exists, replace it with a single `#[test]` that calls `ToolRegistry::global().len()`
and asserts it equals 28 — a count assertion `tool_registry_tests.rs` does not have.

### P2 — Strengthen Test 4 (assertions)

**Action:** Replace `!is_empty()` checks with minimum-count assertions (e.g., `>= 4`).
Replace the hardcoded `"deductive-reasoning"` URI with a dynamic one derived from
`catalog.single_agent_workflows.first()`. This makes the test self-healing against
workflow renames while still proving URI routing.

### P3 — Strengthen Test 1 (directory existence)

**Action:** Add `assert!(cfg.memory_path.is_dir())`, `assert!(cfg.assets_path.is_dir())`,
and `assert!(cfg.database_path.parent().unwrap().is_dir())` after `isolated_config()`.
This turns a "did we set fields correctly" test into a "did we actually prepare the
filesystem" test.

### P4 — Add `T-*` traceability comments

**Action:** Annotate all tests with the relevant task and RTM item. At minimum:
- Tests 1, 3, 4, 5 → `// NFR-2, NFR-3`
- Test 3 → `// FR-22: T-16.1` (DbBackend will own schema init; this test is the regression
  guard for that migration)

### P5 — Consider a shared cross-file mutex or test-env pattern

**Observation:** `contracts.rs` and `tool_registry_tests.rs` both define file-local
`static TEST_MUTEX: Lazy<Mutex<()>>`. Because `Config::override_root()` is process-global
state, two tests from different files running concurrently can corrupt each other's config
state even when both are "holding their mutex." The robust fix is a single shared mutex in
a `tests/support/` module, or use of a test-environment RAII guard that also holds a
cross-module lock. This is a low-frequency race (cargo test parallelism is per-function,
not per-file), but it can produce mysterious CI flakes.

---

## Load-Bearing Score: 4/5

| Test | Load-Bearing |
|------|:------------:|
| `config_honors_override_paths` | ✅ Yes (only Config contract test) |
| `tool_registry_is_case_insensitive_and_exposes_contract_names` | ❌ No (duplicated) |
| `graph_database_creates_shared_schema_tables` | ✅ Yes (only `sqlite_master` enumeration) |
| `assets_bootstrap_and_catalog_are_available` | ✅ Yes (only `read_resource` routing test) |
| `catalog_uses_current_config_assets_path_by_default` | ✅ Yes (unique path-binding contract) |

4 of 5 tests carry unique load. The one that does not (`tool_registry_is_case_insensitive_and_exposes_contract_names`) should be removed without ceremony — its absence would be undetectable behind `tool_registry_tests.rs`.

The most urgent fix is **P0**: add `"concepts"` and `"concept_mentions"` to test 3. A schema
contract test that misses 2/7 core tables provides false confidence. Every other finding here
is a quality improvement; that one is a correctness gap.

[[test-quality]] [[maenifold-rs]] [[contracts-testing]] [[schema-validation]] [[test-coverage]]
