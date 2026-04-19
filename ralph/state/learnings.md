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
- ui_window.rs — 766 prod LOC (2.55x, largest remaining)

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
