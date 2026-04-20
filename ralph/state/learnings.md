# RALPH Learnings

## 2026-04-19 — Session 1 (maenifold-gate LOC compliance)

### What worked
- File-splitting tasks are ideal RALPH targets: mechanical, low risk, clear acceptance criteria
- `lightweight` pipeline (SWE → blue-team → done) is the right fit — no red-team needed for pure code moves
- gpt-5.3-codex SWE agents reliably execute file splits when given precise extraction lists with line numbers
- claude-opus-4.5 blue-team agents consistently verify build + test + LOC in under 60 seconds

### Patterns
- Always check `done/` for ID conflicts before creating new T-200.X tasks (hit T-200.5 collision)
- Gate MCP may fail with "Read-only file system (os error 30)" — fall back to CLI immediately
- SWE agents sometimes pick up pre-existing unstaged changes in `git diff --stat` — verify only expected files changed
- `awk '/^#\[cfg\(test\)\]/{print NR-1}'` is a reliable way to count production LOC

### Delivered
- T-200.2: cache_db.rs 795→234+273 (cache_task_row.rs extracted)
- T-200.3: mcp/mod.rs 333→212+137 (mcp/dispatch.rs extracted)
- T-200.4: cli.rs 487→273+222 (cli_parser.rs extracted)
- T-200.6: tray.rs 358→260+113 (tray_lifecycle.rs extracted)

### Remaining LOC violations (production code only)
- ~~ui_window.rs — 766 prod LOC (2.55x, largest remaining)~~ DONE in Session 3 (T-200.8)

## 2026-04-19 — Session 2 (T-179.35 gate MCP EROFS fix)

### What worked
- TDD pipeline (pending → blue-team → swe → red-team → done) caught a real gap via red-team
- goldeneye red-team correctly identified that ensure_gate_gitignore_entry had the same bare IO error problem
- Remediation cycle (fail → re-implement → re-audit) worked cleanly in one cycle
- Sequential thinking session grounded the root cause analysis before dispatching agents

### Patterns
- Gate task IDs use full filename stem when no `id:` frontmatter exists (e.g., `T-179.35-gate-mcp-read-only-filesystem-error`)
- Gate MCP "Read-only file system" root cause: HTTP server CWD is `/` (macOS read-only APFS). Now fixed with path canonicalization in dispatch.rs
- When auditing error-handling fixes, check ALL write sites in the call path — the first-hit site may differ from the reported site
- Blue-team in TDD first pass: establish baseline + confirm bug + specify test plan (doesn't write code)

### Delivered
- T-179.35: Gate MCP EROFS fix — root validation, path-wrapped errors, 3 new tests (+113 lines, 3 files)

## 2026-04-19 — Session 3 (maenifold-pipeline-driver LOC compliance)

### What worked
- Same file-splitting pattern from Session 1 applied cleanly to a different crate
- SWE completed in ~130s with build verification
- Blue-team completed in ~56s — fast turnaround

### Delivered
- T-200.7: main.rs 379→169+212 (recovery.rs extracted from pipeline-driver)

## 2026-04-19 — Session 4 (maenifold-gate ui_window.rs LOC compliance)

### What worked
- 4-way file split (largest yet) completed cleanly: 766→278+69+223+165
- SWE (gpt-5.3-codex) handled complex cross-module visibility (`pub(crate)` fields for GateUiApp) correctly
- Blue-team confirmed 128/128 tests passed

### Patterns
- When splitting files with cross-references (e.g., `toolbar()` taking `&mut GateUiApp`), struct fields need `pub(crate)` and the struct itself needs `pub(crate)` for sibling modules to import it
- Sequential thinking session up front saved time by identifying the visibility dependency chain before dispatch

### Delivered
- T-200.8: ui_window.rs 766→278+69+223+165 (ui_task_data.rs, ui_filtering.rs, ui_progress.rs extracted)

## 2026-04-19 — Session 5 (maenifold-gate code hygiene + test coverage)

### What worked
- Bundling multiple small hygiene fixes into one lightweight task is efficient
- SWE agents (gpt-5.3-codex) reliably add test modules when given complete struct layouts and test specs
- Blue-team agents consistently verify build + test in under 2 minutes
- Sequential thinking sessions grounding work before dispatch saves time on ID conflicts

### Patterns
- Gate task ID collisions are common when multiple RALPH instances run in parallel — always check todo/ AND done/ right before creating, and be ready to rename
- Gate MCP still returns EROFS on macOS; CLI fallback is reliable
- When the crate has no LOC violations or clippy warnings, focus shifts to test coverage gaps and code hygiene
- In-memory unit tests (no filesystem) are ideal SWE targets — fast, deterministic, no setup

### Delivered
- T-200.10: Code hygiene — test name fix (six→seven), duplicate deserializer removed, clippy allow removed, 3 section format roundtrip tests
- T-200.13: 9 unit tests for TaskStatusDeriver::derive() — core state machine replay logic (131 total tests)
- T-200.15: 5 edge-case tests for TaskSectionParser::parse() — parser robustness (132 total tests)

## 2026-04-19 — Session 5 (maenifold-jockey LOC compliance)

### What worked
- 4 consecutive file-splitting tasks completed without failures: T-200.9, T-200.11, T-200.12, T-200.14
- gpt-5.3-codex SWE agents reliably handle file splits including function extraction (T-200.12 extracted inner loop into new function)
- claude-opus-4.5 blue-team agents provide consistent build+test+LOC verification
- Lightweight pipeline (SWE → blue-team) is ideal for mechanical refactors — average cycle time ~90s per task
- Delegation pattern (trait method → free function) works cleanly for splitting trait impls (T-200.11)

### Patterns
- Check for T-200.X ID collisions across parallel RALPH instances before creating tasks (hit T-200.13 collision)
- When splitting a file with one giant function (e.g., execute.rs), extract the inner loop as a new function rather than trying to move the whole function
- `flush_delta_batch!` macro captured local variables — converting to inline code was simpler than a closure
- Re-export patterns (`pub(crate) use super::new_module::fn;`) preserve existing caller import paths without touching caller files
- Blue-team agents sometimes give terse verdicts — follow up with explicit evidence request if needed

### Delivered
- T-200.9: handlers.rs 857→183+200+251+254 (handler_config, handler_execute, handler_prewarm extracted)
- T-200.11: client.rs 417→298+149 (client_terminal extracted)
- T-200.12: execute.rs 395→182+251 (execute_stream extracted with run_delta_loop)
- T-200.14: cold_start.rs 343→273+79 (cold_start_helpers extracted)

### Remaining LOC violations
- None in maenifold-jockey — all files now under 300 LOC

## 2026-04-19 — Session 6 (maenifold-gate clippy + test coverage)

### What worked
- Clippy violations are ideal lightweight tasks: mechanical, clear acceptance criteria
- Struct extraction (ErrorRowContext, SectionRecord) cleanly resolves too_many_arguments
- Test-only tasks complete fastest: ~90s SWE + ~40s blue-team
- gpt-5.3-codex SWE agents reliably append test code when given exact test bodies
- claude-opus-4.5 blue-team agents consistently verify build + test in under 45s

### Patterns
- Gate MCP still returns EROFS on macOS: CLI fallback is reliable
- SWE agents occasionally touch ralph/ files: restore with git checkout before commit
- Provide exact test code to minimize SWE interpretation variance
- Sequential thinking sessions up front help identify correct task scope

### Delivered
- T-200.16: 3 clippy violations fixed (ErrorRowContext, SectionRecord, redundant import)
- T-200.17: 13 unit tests for state_machine (8) and gate_definition (5)
- T-200.18: 10 MCP dispatch argument parsing tests (164 total tests)

## 2026-04-19 — Session 7 (embedding-models MCP tool + Settings UI)

### What worked
- Full SDD pipeline (SWE → red-team → blue-team) with remediation cycles works reliably for cross-concern features
- goldeneye red-team catches real bugs: dead code branches, silent defaults, install-path mismatches, misleading UX
- Parallel status/list MCP calls via Promise.all in Vue components work cleanly
- Gate CLI fallback reliable when MCP returns EROFS
- gate sync --rebuild needed after pass/fail to update status cache

### Patterns
- SWE unreliable narrator: verify edits persisted — T-179.32 SWE claimed embedding_models.rs fixed but file was untracked (no diff visible), needed careful verification
- gate pass may succeed silently but not update cache — always run gate sync --rebuild before checking status
- For new untracked files, git diff --stat won't show changes — check git status --short for ?? entries
- activeModel computed pattern (find active from list) is better than relying on raw model_filename for UI display
- v-else-if="!error" guard pattern prevents misleading empty-state rendering on MCP failure

### Delivered
- T-179.32: EmbeddingModels MCP tool — status + list operations, curated 6-model catalog, path-aware active detection, non-empty file validation (3 new files, 5 modified)
- T-179.33: SettingsModels.vue — Models section in Settings tab with current model, available models list, status badges, dimension warning (1 new file, 1 modified)

### Remaining
- T-179.34: First-run onboarding wizard — SLC pipeline, exceeds RALPH change-size limits (~2450 LOC reference impl). Needs human-driven sprint with full architect duo.
