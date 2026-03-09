# Sprint Retrospectives — maenifold-rs

## Sprint 10 — Port Parity Completion (Batch 1) — 2026-03-09

### What we delivered
- **T-10.1**: Copied 28 missing workflow JSONs (69/69 parity)
- **T-10.8**: CI now runs workspace-root tests including oracle parity
- **T-10.9**: Config env vars renamed to match C# + 10 missing vars added
- **T-10.10**: RepairConcepts Mode B tests use real assertions (28/28 pass)
- **T-10.12**: readme.md reflects 28/28 implemented tools
- **T-10.13**: TODO.md status fields updated for 16 completed items
- **T-10.14**: docs/DEVELOPMENT.md created
- **T-10.15**: Bundled integration assets copied (7 files)
- **T-10.16**: WikiLink filter reads .wikilink-filter.json (JSON format, matches C#)
- **T-10.17**: Asset catalog cross-repo masking removed

### What worked
- 9-agent parallel execution — 4 researchers + 5 SWEs running concurrently
- Researchers reading ALL .md files caught nuanced gaps (WikiLink filter format drift, catalog masking)
- User's independent audit caught the two compatibility bugs researchers missed
- Batch-commit strategy kept working tree clean between waves

### What didn't work
- SWE agent (T-10.13) rewrote TODO.md instead of doing surgical edits, losing Sprint 10 items — had to re-add manually
- Pre-existing MCP stdio tests (6) and oracle parity tests (12) are failing — not Sprint 10 regressions but they block CI

### Pre-existing failures documented
- 6 MCP stdio handshake failures (`mcp_tests.rs`) — T-10.18
- 12 oracle parity test failures (`tests/compat/*`) — T-10.19

### Remaining Sprint 10 backlog (deferred)
- T-10.2-4: MCP native resources + notifications + auto-start watchers
- T-10.5-7: Oracle parity tests for 6 tools
- T-10.11: Sprint 9 RTM gaps (4 tests)
- T-10.18: Fix MCP stdio test failures
- T-10.19: Fix oracle parity test failures

### Action items
1. Fix pre-existing test failures before adding new features (T-10.18, T-10.19)
2. Instruct SWE agents to use surgical edits, not file rewrites, on shared docs
3. MCP resource parity (T-10.2-4) is the biggest remaining product gap
