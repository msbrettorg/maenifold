# RALPH Thoughts

Thoughts and ideas for future improvements belong here.
Stuff you learn should go in ralph/state/learnings.md
This isn't a work log, it's a place for ideas that aren't yet ready to be implemented but are worth remembering and revisiting later.

## Iteration 20260425-0759

### Shipped (9 tasks)
- **T-220.29** ✅ Align tray-icon workspace version (SDD, 0 remediation)
- **T-220.23** ✅ Replace window.confirm with AlertDialog (SDD, 0 remediation)
- **T-220.25** ✅ Remove manual tokenizer fallback (SDD, 1 remediation: path leak)
- **T-220.26** ✅ Prune 11 unused dependencies (SDD, 0 remediation)
- **T-218.5** ✅ CDP click-through matrix (SDD, 9/10 items PASS)
- **T-220.19** ✅ Harden design-system audit enforcement (SDD, 0 remediation)
- **T-220.21** ✅ Update UAT and tab tests for Teams (SDD, 0 remediation)
- **T-220.20** ✅ Sanction file input wrapper (SDD, 0 remediation)
- **T-220.18** ✅ Surface Teams unread errors (SDD, 0 remediation)

### Stats: 504→513 done (+9 shipped)

### T-220.27 (serde_yaml) needs epic decomposition
- serde_yaml has 80+ usage sites across core, gate, pipeline-driver
- Used heavily for YAML frontmatter in markdown files (core feature)
- Candidates: serde_yml (maintained fork), yaml-rust2 + serde bridge
- Should be decomposed into: (1) research/evaluate replacements, (2) migrate core/markdown, (3) migrate workflow, (4) migrate gate, (5) migrate pipeline-driver, (6) cleanup
- NOT a single SWE task — this is a cross-crate migration

### Wave 2 candidates after current wave ships
- T-220.19 + T-220.20 — audit script hardening (frontend, sequential dependency)
- T-220.28 — reqwest alignment (investigate scope first)
- T-218.20 — gateway resource registry (complex backend, high value)
- T-218.24 — ACP resume hardening (complex, high risk)

### CDP observations from this iteration
- All major tabs render: Home, Agents, Gate, Knowledge, Teams, Settings
- Slash command menu works (single /clear command visible for Codex agent)
- Gate Done tab click properly updates detail panel (T-219.1 fix confirmed)
- Schedules settings renders with table and empty state
- Image/Audio buttons disabled (expected — need model support)

## Iteration 20260425-0837

### In-Flight
- **T-220.16** ✅ SWE done, ✅ red-team PASS, 🔄 blue-team running
- **T-220.7** ✅ SWE done, ❌ red-team FAIL (null hashes), ✅ remediated with real hashes, 🔄 red-team re-audit running
- **T-220.24** ✅ SWE done, 🔄 red-team running
- **T-220.10** ❌ SWE failed (ort link error without download-binaries). Needs research into ort load-dynamic feature or feature-gating entire vector module.

### Grooming
- T-218.19 deprioritized to P2 (unreachable code path per duo review)

### T-220.10 research needed
- `ort` crate's `download-binaries` is required for build on machines without system ONNX Runtime
- Possible approaches: (1) `ort/load-dynamic` feature for runtime dlopen, (2) feature-gate entire vector module, (3) accept status quo and document
- Should probably change pipeline from SDD to GSD since this needs research first

### Wave 2 dispatched
- **T-218.27** — Stdio MCP server-to-client request compliance (maenifold-core, SDD)
  - Found the gap: server requests (method + id) silently dropped in read_response loop
  - Fix: detect and respond with JSON-RPC -32601 method-not-found

### Grooming done
- T-218.39 partially superseded by T-220.24 (lastError store ref), remaining: UI display + CDP smoke
- T-218.19 confirmed P2 (unreachable code path)
- Created T-222.1 follow-up for voice companion absolute path hardening

### Iteration stats so far
- 3 shipped: T-220.16, T-220.7, T-220.24
- 1 failed: T-220.10 (ort link error, needs research)
- 1 in-flight: T-218.27 (SWE running)
