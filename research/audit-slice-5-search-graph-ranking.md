# Parity Audit Slice 5: Search, Graph Traversal, and Ranking

**Date**: 2025-07-07
**Traceability**: T-10.19 (pre-existing oracle parity), FR-7 (search), FR-8 (vec0), NFR-1 (oracle parity)
**Sequential thinking session**: session-1773104046670-38000, branch: audit-slice-5-search

## Executive Summary

Exhaustive comparison of search, graph traversal, sync, and ranking algorithms across 8 C# files and 4 Rust files reveals **3 blocking, 4 important, and 3 cosmetic** parity gaps. The single largest finding: the Rust `SearchMemories` function does not use vector search at all — its "semanticScore" is a string-contains heuristic, not embedding-based similarity. This invalidates Hybrid and Semantic search modes versus the C# reference. Text scoring also uses a fundamentally different formula, and `FindSimilarConcepts` is missing 5 quality guards.

---

## Findings

### 1. SearchMemories — Semantic/Hybrid Architecture

```
Algorithm: SearchMemories Hybrid Search
C# implementation: Two-pipeline architecture:
  1. Vector pipeline: GetVectorSearchResults() → vec_distance_cosine() on vec_memory_files
     SQL: SELECT file_path, vec_distance_cosine(embedding, @embedding) as distance
          FROM vec_memory_files WHERE embedding IS NOT NULL ORDER BY distance LIMIT @max
     Score normalization: 1.0 - ((dist-min)/(max-min)), epsilon 1e-12
     Candidate limit: min(maxResults*5, 200)
  2. Text pipeline: GetTextSearchResults() → occurrence counting
  3. RRF fusion: 1/(k+i+1) with k=60, then × decayWeight
Rust implementation: Single-pipeline architecture:
  1. Iterates file_content rows from DB
  2. semanticScore = 1.0 if content.contains(query_slug) else 0.0 — NO VECTOR SEARCH
  3. "Fusion" = text_score + semantic_score, then RRF on combined set
Status: DRIFT
Severity: blocking
Details: The Rust semantic channel is a string-contains heuristic, not embedding-based
  similarity. For any query where relevant files don't contain the exact query slug,
  semanticScore=0 in Rust vs a real cosine-distance similarity in C#. Hybrid mode
  is fundamentally degraded. The vec_memory_files table IS populated during sync,
  but never queried during search.
```

### 2. SearchMemories — Text Scoring Formula

```
Algorithm: Text Search Scoring
C# implementation:
  score = Σ per term (
    contentLower.Split(term).Length - 1        // occurrence count
    + (titleLower.Contains(term) ? 5 : 0)     // title bonus per term
  )
  Raw score examples: query "graph traversal" → could score 15+ (3 "graph" + 5 title + 2 "traversal" + 5 title)
Rust implementation:
  text_matches = query_terms.filter(|t| title.contains(t) || content.contains(t)).count()
  text_score = text_matches / (query_terms.len() * 50.0)
  Score examples: same query → max 2/(2*50) = 0.02 regardless of occurrence count
Status: DRIFT
Severity: blocking
Details: C# counts all occurrences of each term and adds +5 title bonus per term found.
  Rust counts how many DISTINCT terms match (binary per term) and divides by terms*50.
  This produces completely different ranking orders. A document mentioning "graph" 50 times
  ranks the same as one mentioning it once in Rust, but 50× higher in C#.
```

### 3. SearchMemories — RRF Fusion Constants

```
Algorithm: Reciprocal Rank Fusion
C# implementation: k=60, formula: 1.0/(k + i + 1) where i is 0-indexed loop variable
  Text score normalization: min(score/100.0, 1.0)
  Pre-fusion multiplier: 3 (retrieves pageSize×3 candidates per pipeline)
  Final: fusedScore = rrfScore × decayWeight
Rust implementation: k=60, formula: 1.0/(60.0 + rank) where rank is 1-indexed
  No text score normalization (different scoring system)
  No pre-fusion multiplier (single pass)
  Final: fusedScore = fused × decayWeight
Status: MATCH (for RRF formula itself)
Severity: cosmetic (pre-fusion multiplier difference)
Details: RRF formula produces same values: C# first rank = 1/(60+0+1)=1/61,
  Rust first rank = 1/(60+1)=1/61. The pre-fusion multiplier affects candidate
  count but not the formula. However, since inputs differ (due to findings #1/#2),
  outputs will differ despite matching formula.
```

### 4. SearchMemories — Snippet Extraction

```
Algorithm: Snippet Extraction
C# implementation:
  start = max(0, firstTermIndex - 20)      // 20 chars before first match
  length = min(200, content.Length - start)  // up to 200 chars total
  Whitespace normalization: replace \r\n and \n with space
Rust implementation:
  start = index.saturating_sub(25)          // 25 chars before first match
  end = (index + 48).min(content.len())     // 48 chars after match position
  No whitespace normalization
Status: DRIFT
Severity: important
Details: C# produces ~200 char snippets, Rust produces ~73 char snippets.
  Different context windows (20 before vs 25 before). Different total length.
  Rust snippets are significantly shorter and may be less informative.
```

### 5. FindSimilarConcepts — Quality Guards

```
Algorithm: FindSimilarConcepts Input Validation & Output Quality
C# implementation: 5 quality guards before/after vector search:
  1. IsLowInformationQuery: rejects whitespace-only, punctuation-only, no letters/digits
  2. ContainsBracketLikeDelimiter: rejects 21 Unicode bracket types (CJK, math, dingbat)
  3. Max concept name length: 256 chars
  4. IsValidEmbeddingForSearch: rejects NaN, Inf, near-zero magnitude (sumSq > 1e-12)
  5. IsSimilarityPlateau: detects degenerate results (top-10 variance ≤ 1e-9, near-zero count ≥ topK-1)
Rust implementation: None of these guards present.
  Only validates: maxResults range 1..=50
Status: MISSING
Severity: blocking
Details: Without these guards, Rust may return garbage results for edge-case queries
  (emoji-only, bracket-heavy, zero-norm embeddings). The plateau detection is important
  for signaling when vec0 hasn't been properly seeded.
```

### 6. FindSimilarConcepts — Core Algorithm

```
Algorithm: FindSimilarConcepts Vector Search
C# implementation:
  SQL: SELECT concept_name, vec_distance_cosine(embedding, @embedding) as distance
       FROM vec_concepts WHERE concept_name != @concept AND concept_name != ''
       AND instr(concept_name, '[') = 0 AND instr(concept_name, ']') = 0
       ORDER BY distance LIMIT @maxResults
  Similarity: 1.0 / (1.0 + max(0, distance)), clamped [0, 1]
Rust implementation:
  Identical SQL query and similarity formula
  Graph fallback: co_occurrence_count / max_count for normalization
Status: MATCH (core algorithm)
Severity: n/a
Details: Core SQL, distance-to-similarity conversion, and graph fallback all match.
```

### 7. BuildContext — Graph Traversal

```
Algorithm: BuildContext Graph Traversal
C# implementation:
  Default depth: 2, default maxEntities: 20
  Direct relations SQL: SELECT CASE WHEN concept_a = @name THEN concept_b ELSE concept_a END,
    co_occurrence_count, source_files FROM concept_graph WHERE concept_a = @name OR concept_b = @name
    ORDER BY co_occurrence_count DESC LIMIT @maxEntities
  Scoring: count × avgDecayWeight (weighted score)
  Content preview: targetLength=200, tolerance=50, smart sentence boundary detection
  Community enrichment: min shared neighbors, min overlap, max siblings (all configurable)
Rust implementation:
  Default depth: 2, default maxEntities: 20
  Similar SQL: SELECT concept_a, concept_b, COALESCE(co_occurrence_count, 0.0),
    COALESCE(source_files, '') FROM concept_graph WHERE concept_a = ?1 OR concept_b = ?1
    ORDER BY co_occurrence_count DESC, concept_a ASC, concept_b ASC
  Scoring: co_occurrence_count × decayWeight (similar weighted score)
  Content preview: max 500 chars truncation (simpler than C# smart boundaries)
  Community enrichment: min_shared=3, min_overlap=0.4, max_siblings=10 (configurable)
Status: MATCH (structural), DRIFT (content preview)
Severity: cosmetic
Details: Graph traversal logic, depth/entity limits, and community enrichment match.
  Content preview uses simpler truncation (500 hard limit) vs C#'s smart sentence
  boundary detection (200 target + 50 tolerance). SQL has minor ordering differences
  (Rust adds secondary sort by concept name for determinism).
```

### 8. Decay Calculation

```
Algorithm: Decay Weight Calculation
C# implementation:
  Exponential: 0.5^((age - grace) / halfLife), clamped [0, 1]
  Power-law (ACT-R): a × t^(-b), a=1.0, b=0.5, clamped [0, 1]
  Default function: "power-law" (from Config)
  Grace periods: sequential=7d, workflow=14d, default=28d, halfLife=30d
  Path matching: "thinking/sequential", "/sequential/", "thinking/compaction" → 7d
                  "thinking/workflows", "/workflows/" → 14d
  Assumption decay: ALWAYS uses exponential (ignores configured function)
  No-DB fallback: reads frontmatter created date from disk
Rust implementation:
  Exponential: 0.5^((age - grace) / halfLife), clamped [0, 1]  ← MATCH
  Power-law: 1.0 × effective_age^(-0.5), clamped [0, 1]        ← MATCH
  Default function: "power-law" (from Config)                    ← MATCH
  Grace periods: sequential=7d, workflow=14d, default=28d        ← MATCH
  Path matching: "/thinking/sequential/" → 7d                    ← DRIFT (missing compaction, /sequential/)
                  "/thinking/workflow/", "/thinking/workflows/" → 14d  ← DRIFT (missing /workflows/ standalone)
  Assumption decay: uses CONFIGURED function (power-law if default)     ← DRIFT
  No-DB fallback: returns 1.0 (no decay)                               ← DRIFT
Status: DRIFT
Severity: important (3 sub-drifts)
Details:
  a) Path matching misses "thinking/compaction" and standalone "/sequential/" patterns.
     Compaction files get 28d grace instead of 7d.
  b) Assumption decay always uses exponential in C#; Rust uses config'd function.
     With default power-law config, assumptions decay differently.
  c) When no last_accessed timestamp in DB, Rust returns 1.0 (no decay).
     C# falls back to reading created date from disk frontmatter.
     Newly-synced files never decay in Rust until they get a DB timestamp.
```

### 9. Sync — WikiLink Extraction & Graph Updates

```
Algorithm: Incremental Sync
C# implementation:
  Hash guard: MD5 base64, size guard before hash
  WikiLink extraction: regex [[concept]] from content
  Concept normalization: to_lowercase
  Edge proximity weight: 1.0 (same named section), 0.5 (cross-section/preamble)
  Edge comparison tolerance: 1e-10 (skip write if unchanged)
  Orphan cleanup: DELETE concepts WHERE NOT IN concept_mentions
  SQLite chunk size: 900 parameters
  Edge update: UPDATE co_occurrence_count, source_files WHERE concept_a=? AND concept_b=?
Rust implementation:
  Hash guard: MD5 base64, mtime guard before hash
  WikiLink extraction: extract_wikilinks() from markdown module
  Concept normalization: to_ascii_lowercase
  Edge proximity weight: 1.0 (same named section), 0.5 (cross-section) ← MATCH
  Edge comparison tolerance: none (always writes)                       ← cosmetic drift
  Orphan cleanup: DELETE concepts WHERE NOT EXISTS concept_mentions      ← MATCH
  SQLite chunk size: 900 parameters                                     ← MATCH
  Edge update: ON CONFLICT DO UPDATE (upsert pattern)                   ← functionally equivalent
Status: MATCH (structural)
Severity: cosmetic
Details: Minor difference in edge write frequency (Rust writes more often due to
  missing epsilon guard). C# uses size guard before MD5; Rust uses mtime guard.
  Both achieve same end state. Normalization: C# uses ToLowerInvariant(), Rust uses
  to_ascii_lowercase() — difference only for non-ASCII characters in concepts.
```

### 10. Community Detection — Louvain & NMI

```
Algorithm: Louvain Community Detection + NMI Drift
C# implementation:
  Modularity: Q = (1/2m) × Σ[A_ij - γ(k_i×k_j)/(2m)] × δ(c_i,c_j)
  Gamma: 1.0 (configurable via MAENIFOLD_LOUVAIN_GAMMA)
  NMI: 2×I(X;Y) / (H(X)+H(Y)), range [0,1]
  Drift threshold: Config.CommunityNmiDriftThreshold (default 0.8)
  Edge weight: Σ(proximity[file] × decay_weight_for_file(file))
  EPSILON: (implicit in divide-by-zero guards)
Rust implementation:
  Modularity: Q = Σ[(e_c/(2m)) - γ(d_c/(2m))²]  ← equivalent formulation
  Gamma: 1.0 (configurable)                        ← MATCH
  NMI: 2×mutual_info / (H_left + H_right)          ← MATCH
  Drift threshold: 0.8 (from env var)              ← MATCH
  Edge weight: sum of source_file_weights or fallback to co_occurrence_count
  EPSILON: 1e-12 (explicit constant)
Status: MATCH
Severity: cosmetic (edge weight aggregation detail)
Details: Core algorithms match. Modularity formulations are mathematically equivalent
  (per-edge vs per-community aggregation). NMI drift threshold matches. Minor difference
  in edge weight calculation: C# explicitly multiplies proximity × file_decay for each
  source file; Rust reads pre-computed weights from source_files JSON. Same end result
  when source_files JSON is populated by sync.
```

### 11. RepairConcepts

```
Algorithm: RepairConcepts (Mode A + Mode B)
C# implementation:
  Mode A regex: \[\[variant\]\] with IgnoreCase → replace with [[canonical]]
  Mode B: Markdig AST walk, skip CodeBlock/FencedCodeBlock/CodeInline/LinkInline
  Anti-duplicate: check 2 chars before for "[["
  Apply order: reverse sort by position
  Semantic validation: cosine_similarity(embedding_a, embedding_b) >= 0.7
  Split chars: , ; |
Rust implementation:
  Mode A regex: (?i)\[\[variant\]\] → replace with [[canonical]]      ← MATCH
  Mode B: pulldown-cmark AST walk, skip CodeBlock/Link depth counters
  Anti-duplicate: find_existing_wikilink_ranges()
  Apply order: reverse sort by position                                ← MATCH
  Semantic validation: cosine_similarity >= 0.7                        ← MATCH
  Split chars: , ; |                                                   ← MATCH
Status: MATCH
Severity: n/a
Details: Both implementations are functionally equivalent. Different markdown parsers
  (Markdig vs pulldown-cmark) but same AST node filtering logic. Different anti-duplicate
  approaches but same protection goal.
```

### 12. AnalyzeConceptCorruption

```
Algorithm: Concept Variant Classification
C# implementation:
  File Paths: starts_with('/') || contains('.')
  Singular/Plural: ends_with('s') && !ends_with('ss')
  Compound: contains('-')
  With Suffix: contains "system"/"framework"/"architecture"/"tool" (case-insensitive)
  Other: everything else
  Max results: 50
Rust implementation: Identical classification logic, identical keywords, max 50
Status: MATCH
Severity: n/a
Details: Perfect parity.
```

### 13. ExtractConceptsFromFile

```
Algorithm: WikiLink Extraction
C# implementation: Regex \[\[([^\]]+)\]\] on content, normalize, deduplicate
Rust implementation: extract_wikilinks(content), to_ascii_lowercase, BTreeSet dedupe
Status: MATCH
Severity: n/a
Details: Both extract [[concept]] patterns and deduplicate. Minor difference:
  C# uses ToLowerInvariant, Rust uses to_ascii_lowercase (ASCII-only lowering).
```

---

## Stopwords Comparison

```
C# (41 words): a, an, and, are, as, at, be, but, by, for, if, in, into, is, it,
  no, not, of, on, or, such, that, the, their, then, there, these, they, this,
  to, was, will, with, we, you, your, our, from
Rust (40 words): a, an, and, are, as, at, be, but, by, for, if, in, into, is, it,
  no, not, of, on, or, such, that, the, their, then, there, these, they, this,
  to, was, will, with, we, you, your, our, from
Status: MATCH
Details: Same 40 stopwords. C# has case-insensitive HashSet; Rust lowercases before
  comparison. Functionally identical.
```

---

## Configuration Defaults Comparison

| Config Key | C# Default | Rust Default | Status |
|-----------|-----------|-------------|--------|
| DECAY_GRACE_DAYS_SEQUENTIAL | 7 | 7 | MATCH |
| DECAY_GRACE_DAYS_WORKFLOWS | 14 | 14 | MATCH |
| DECAY_GRACE_DAYS_DEFAULT | 28 | 28 | MATCH |
| DECAY_HALF_LIFE_DAYS | 30 | 30 | MATCH |
| DECAY_FUNCTION | "power-law" | "power-law" | MATCH |
| LOUVAIN_GAMMA | 1.0 | 1.0 | MATCH |
| COMMUNITY_MIN_SHARED | 3 | 3 | MATCH |
| COMMUNITY_MIN_OVERLAP | 0.4 | 0.4 | MATCH |
| COMMUNITY_MAX_SIBLINGS | 10 | 10 | MATCH |
| COMMUNITY_NMI_THRESHOLD | 0.8 | 0.8 | MATCH |
| COMMUNITY_DEBOUNCE_MS | 2000 | 2000 | MATCH |
| SESSION_ABANDON_MINUTES | 30 | (not in config.rs) | MISSING |
| AUTO_SYNC | true | true | MATCH |
| INCREMENTAL_OPTIMIZE_EVERY | 40 | 40 | MATCH |
| INCREMENTAL_VACUUM_MINUTES | 720 | 720 | MATCH |
| SESSION_CLEANUP | true | true | MATCH |
| SYNC_LOGGING | true | true | MATCH |
| WATCHER_BUFFER | 65536 | 65536 | MATCH |
| SQLITE_BUSY_TIMEOUT | 5000 | 5000 | MATCH |
| EMBEDDING_LOGS | false | false | MATCH |
| VECTOR_LOGS | false | false | MATCH |

---

## Recommended Backlog Items

### T-NEW-1: Integrate vector search into SearchMemories (BLOCKING)
- Add a vector search pipeline that queries `vec_memory_files` using `vec_distance_cosine()`
- Implement min-max score normalization with epsilon 1e-12
- Apply candidate limit `min(maxResults*5, 200)`
- Wire into RRF fusion alongside existing text pipeline
- **RTM**: FR-7, NFR-1

### T-NEW-2: Fix text scoring formula to match C# (BLOCKING)
- Replace binary term-match counting with occurrence-based counting
- Add +5 title bonus per term found in title
- Remove the `/50.0` normalization (C# returns raw scores; normalization happens in fusion as `min(score/100, 1.0)`)
- **RTM**: FR-7, NFR-1

### T-NEW-3: Add FindSimilarConcepts quality guards (BLOCKING)
- Implement plateau detection (top-10 variance ≤ 1e-9)
- Implement embedding validity check (no NaN/Inf, sumSq > 1e-12)
- Implement low-information query rejection
- Implement bracket-like delimiter detection (21 Unicode ranges)
- Implement max concept name length (256 chars)
- **RTM**: NFR-1

### T-NEW-4: Fix snippet extraction to match C# (IMPORTANT)
- Change context window from 25-before/48-after to 20-before/200-total
- Add whitespace normalization (replace \r\n and \n with space)
- **RTM**: FR-7, NFR-1

### T-NEW-5: Fix decay path matching gaps (IMPORTANT)
- Add `thinking/compaction` to sequential grace period matching
- Add standalone `/sequential/` pattern
- Add standalone `/workflows/` pattern
- **RTM**: NFR-1

### T-NEW-6: Fix assumption decay function selection (IMPORTANT)
- Always use exponential decay for assumptions regardless of configured function
- Match C# `GetAssumptionDecayWeight()` which hardcodes `CalculateExponentialDecay()`
- **RTM**: NFR-1

### T-NEW-7: Add disk fallback for decay when no DB timestamp (IMPORTANT)
- When `last_accessed` is None in DB, fall back to reading frontmatter `created` from disk
- Currently returns 1.0 (no decay), should compute real decay from file age
- **RTM**: NFR-1

---

## Sources

| File | Purpose |
|------|---------|
| C# `MemorySearchTools.cs` | Hybrid/Semantic/FullText search orchestration |
| C# `MemorySearchTools.Fusion.cs` | RRF fusion, decay weighting |
| C# `MemorySearchTools.Text.cs` | Text scoring, stopwords, snippet extraction |
| C# `MemorySearchTools.Vector.cs` | Vector search via vec0, score normalization |
| C# `VectorSearchTools.cs` | FindSimilarConcepts, quality guards |
| C# `GraphTools.cs` | BuildContext graph traversal, community enrichment |
| C# `ConceptSync.cs` | Sync logic, edge updates, proximity weights |
| C# `DecayCalculator.cs` | All decay formulas, grace periods, status weights |
| C# `CommunityDetection.cs` | Louvain, NMI, edge weight calculation |
| C# `ConceptRepairTool.cs` | RepairConcepts Mode A/B, semantic validation |
| C# `Config.cs` | All configuration defaults |
| Rust `search.rs` | SearchMemories, BuildContext, Sync, Repair, Analyze |
| Rust `community.rs` | Louvain, NMI, modularity |
| Rust `vector.rs` | Embedding generation, cosine similarity |
| Rust `memory.rs` | Decay calculation, grace periods |
| Rust `config.rs` | Configuration defaults |
