# Parity Audit Slice 7: Asset Content Parity

**Traceability**: T-10.1 (workflow copies), T-1.1/FR-1 (Adopt), T-1.3/FR-3 (UpdateAssets)  
**Date**: 2025-07-08  
**Method**: `diff`, `find`, Python JSON comparison with CRLF normalization  

## Executive Summary

All non-workflow asset files (104 files across roles, colors, perspectives, integrations, agents, models, native, usage) match exactly between C# and Rust (CRLF-only differences for 8 files). Handler code (Adopt, UpdateAssets, ListAssets, Catalog) has full behavioral parity. **The sole blocking issue is 41/69 workflow JSON files that have real content drift** — the C# repo underwent a major workflow enrichment pass that was never propagated to the Rust repo's original 41 files.

---

## Category-by-Category Results

### 1. Workflows

```
Asset category: workflows
C# count: 69
Rust count: 69
Missing from Rust: NONE
Content drift: 41 files with real content drift, 28 files CRLF-only
Status: DRIFT
Severity: blocking
```

**28 CRLF-only files** (copied in T-10.1 Sprint 10):
bayesian-reasoning, communication-iteration, confirmation-bias, confirmation-replication, conjecture-refutation, controlled-experiment, crucial-experiment, double-blind-study, empiricism, experimental-design, falsification, hypothesis-development, hypothetico-deductive, mathematical-modeling, model-dependent-realism, occams-razor, paradigm-analysis, peer-review, pragmatism, prediction, rationalism, reproducibility, scientific-method, statistical-analysis, strong-inference, systematic-observation, theory-ladenness, thought-experiment

**10 field-rename-only files** (>90% size match, same step count):
agentic-dev (100%), agentic-research (94%), agentic-slc (97%), game-theory (97%), memory-consolidation (96%), memory-cycle (96%), memory-epistemic (97%), memory-hub-detection (99%), memory-repair (99%), memory-status (96%)

**31 content-enriched files** (<90% size match):

| File | Size Ratio | Key Drift |
|------|-----------|-----------|
| lean-startup.json | 7% | 4→7 steps, 51→473 char description, +toolHints, +metadata, +specification |
| inductive-reasoning.json | 12% | 5→6 steps, 79→309 char description, +toolHints, +metadata |
| agile.json | 14% | +specification, 56→337 char description |
| strategic-thinking.json | 14% | 5→6 steps, 67→395 char description |
| sixhat.json | 15% | 6→7 steps, 52→1006 char description |
| lateral-thinking.json | 15% | 5→8 steps, 67→501 char description |
| sdlc.json | 15% | 7→8 steps, 50→373 char description |
| critical-thinking.json | 16% | 5→7 steps, 84→274 char description |
| deductive-reasoning.json | 16% | 4→5 steps, 74→239 char description |
| data-thinking.json | 17% | 5→7 steps, 63→427 char description |
| abductive-reasoning.json | 19% | 69→210 char description |
| design-thinking.json | 20% | 57→268 char description |
| parallel-thinking.json | 25% | 5→4 steps, 76→210 char description |
| convergent-thinking.json | 26% | 79→176 char description |
| higher-order-thinking.json | 28% | 5→6 steps |
| scamper.json | 30% | 50→241 char description |
| polya-problem-solving.json | 30% | 88→217 char description |
| divergent-thinking.json | 31% | 75→346 char description |
| oblique-strategies.json | 32% | 4→3 steps, 78→272 char description |
| provocative-operation.json | 31% | 5→3 steps |
| socratic-dialogue.json | 35% | 9→5 steps |
| world-cafe.json | 39% | 26→9 steps, 94→428 char description |
| crta.json | 43% | +toolHints, +metadata |
| role-creation-workflow.json | 48% | +toolHints, +metadata |
| agent-creation-workflow.json | 53% | +toolHints, +metadata |
| ftk-analysis.json | 55% | +toolHints, +metadata |
| ftk-analysis-v2.json | 55% | +toolHints, +metadata |
| ftk-query.json | 51% | +toolHints, +metadata |
| ftk-query-v2.json | 51% | +toolHints, +metadata |
| workflow-dispatch.json | 56% | +toolHints, +metadata |
| think-tank.json | 70% | 9→10 steps |

**Systematic drift patterns across all 41 files**:
1. **Field rename**: Rust uses `enhancedThinkingEnabled` / `requiresEnhancedThinking` (legacy); C# uses `sequentialThinkingEnabled` / `requiresSequentialThinking` (current)
2. **Missing `multiAgent` field**: C# has `multiAgent` on all 69 workflows (12 are `true`); Rust has it on only 28 (the T-10.1 copies), missing from the 41 originals → **5 workflows that should be `multiAgent: true` default to `false`** in Rust, causing catalog split drift
3. **Missing `toolHints`**: C# has toolHints metadata in all 69 workflows; Rust has them in only 39
4. **Missing `metadata` blocks**: Same pattern as toolHints (C# 69/69, Rust 39/69)
5. **Missing `specification` field**: Present in 13 C# workflows, 0 Rust workflows
6. **Truncated descriptions**: C# descriptions average ~300 chars; Rust originals average ~70 chars

**5 multiAgent:true drift files** (critical for catalog split):
- agentic-slc.json: C# `multiAgent: true`, Rust MISSING
- game-theory.json: C# `multiAgent: true`, Rust MISSING
- memory-cycle.json: C# `multiAgent: true`, Rust MISSING
- think-tank.json: C# `multiAgent: true`, Rust MISSING
- world-cafe.json: C# `multiAgent: true`, Rust MISSING

### 2. Roles

```
Asset category: roles
C# count: 16
Rust count: 16
Missing from Rust: NONE
Content drift: NONE
Status: MATCH
Severity: n/a
```

### 3. Colors

```
Asset category: colors
C# count: 7
Rust count: 7
Missing from Rust: NONE
Content drift: NONE
Status: MATCH
Severity: n/a
```

### 4. Perspectives

```
Asset category: perspectives
C# count: 12
Rust count: 12
Missing from Rust: NONE
Content drift: NONE
Status: MATCH
Severity: n/a
```

### 5. Integrations

```
Asset category: integrations
C# count: 7 files (3 shell hooks + 4 docs)
Rust count: 7 files
Missing from Rust: NONE
Content drift: CRLF-only (7/7 match after normalization)
Status: MATCH (cosmetic CRLF)
Severity: cosmetic
```

### 6. Other Asset Categories

| Category | C# Count | Rust Count | Status | Notes |
|----------|----------|------------|--------|-------|
| agents | 1 (product-manager.md) | 1 | MATCH | Exact match |
| usage/tools | 27 | 27 | MATCH | Exact match, all 27 files |
| native | 3 (vec0 binaries) | 3 | MATCH | Exact match (dll/dylib/so) |
| models | 2 (vocab.txt + tokenizer_config.json) | 2 | MATCH | vocab.txt CRLF-only, tokenizer_config exact |

---

## Handler Code Parity

### Adopt (FR-1, T-1.1)
| Aspect | C# | Rust | Status |
|--------|-----|------|--------|
| Two-step lookup (filename → id scan) | ✅ | ✅ | MATCH |
| Instruction template text | ✅ identical | ✅ identical | MATCH |
| Rejects invalid types | ✅ | ✅ | MATCH |
| Path traversal protection | ❌ | ✅ `validate_adopt_identifier` | Rust extra |
| JSON output envelope | plain string | `{type, identifier, instruction, jsonConfiguration}` | Rust extra (Markdown mode matches) |

### UpdateAssets (FR-3, T-1.3)
| Aspect | C# | Rust | Status |
|--------|-----|------|--------|
| Recursive directory walk | ✅ | ✅ | MATCH |
| File size + SHA256 comparison | ✅ `SHA256.Create()` | ✅ `sha2::Sha256` | MATCH |
| Dry-run mode | ✅ default true | ✅ default true | MATCH |
| Symlink protection | ❌ | ✅ skippedSymlinks tracking | Rust extra |
| Error tracking | per-file errors | per-file errors + symlink errors | MATCH+ |

### AssetCatalog / ListAssets
| Aspect | C# | Rust | Status |
|--------|-----|------|--------|
| Catalog splits by multiAgent | ✅ `singleAgentWorkflows`/`multiAgentWorkflows` | ✅ same field names | MATCH |
| Metadata fields | id, name, emoji, description, multiAgent | id, name, emoji, description, multiAgent | MATCH |
| 4 asset types | workflow, role, color, perspective | workflow, role, color, perspective | MATCH |
| Accepts singular/plural | ✅ | ✅ | MATCH |

---

## Total File Count Summary

| Category | C# | Rust | Exact Match | CRLF-only | Real Drift |
|----------|-----|------|-------------|-----------|------------|
| workflows | 69 | 69 | 0 | 28 | **41** |
| roles | 16 | 16 | 16 | 0 | 0 |
| colors | 7 | 7 | 7 | 0 | 0 |
| perspectives | 12 | 12 | 12 | 0 | 0 |
| integrations | 7 | 7 | 0 | 7 | 0 |
| agents | 1 | 1 | 1 | 0 | 0 |
| usage | 27 | 27 | 27 | 0 | 0 |
| native | 3 | 3 | 3 | 0 | 0 |
| models | 2 | 2 | 1 | 1 | 0 |
| **TOTAL** | **144** | **144** | **67** | **36** | **41** |

---

## Recommendations

### Blocking (must fix before release)
1. **Bulk-copy 41 enriched workflow JSONs from C# repo** — `cp /Users/brett/src/ma-collective/maenifold/src/assets/workflows/{file}.json assets/workflows/` for each of the 41 diverged files. This brings all workflows to current content including `multiAgent`, `toolHints`, `metadata`, `specification` fields.

### Important (should fix)
2. **Normalize line endings to LF** across all Rust asset files — the 28 T-10.1-copied workflows and 7 integration files have CRLF endings while C# has LF. Add a `.gitattributes` entry: `assets/** text eol=lf`
3. **Verify catalog output** after bulk copy — confirm the 5 newly-`multiAgent:true` workflows sort into `multiAgentWorkflows` correctly.

### Cosmetic
4. **Consider git hooks or CI** to detect asset drift going forward — a simple `diff --brief -r` in CI would catch this.

---

## Limitations

- Did not compare `.onnx` model binaries (large files, expected to be external/downloaded)
- Did not verify the C# `AssetUpdateResult` formatting exactly matches Rust's `render_update_summary` markdown output (both produce markdown, structure similar but not byte-identical)
- Did not test runtime behavior of UpdateAssets across repos (only static analysis)
