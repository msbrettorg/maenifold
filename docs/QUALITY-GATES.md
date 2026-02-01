# Quality Gates: BuildContext and FindSimilarConcepts

**Version**: 1.0
**Date**: 2026-02-01
**Traceability**: T-QUAL-GATE-001, T-QUAL-GATE-002, PRD FR-7.3, FR-7.4, NFR-7.4.2

---

## Purpose

This document defines release-blocking quality checks for graph retrieval operations. These gates detect regressions in retrieval quality before release.

---

## T-QUAL-GATE-001: Evaluation Query Suite

### Test Concepts (12)

| Query | Category | Domain | Expected Behavior |
|-------|----------|--------|-------------------|
| `maenifold` | hub | system | BuildContext returns 30+ relations; FindSimilar shows variants (maenifold-skill, maenifold-tools) |
| `knowledge-graph` | hub | graph | BuildContext returns 30+ relations; high similarity (>0.8) to `knowledge-graphs` |
| `workflow` | hub | system | BuildContext returns 30+ relations spanning roles and tools |
| `sequential-thinking` | hub | reasoning | High similarity (>0.7) to parallel-thinking, divergent-thinking |
| `memory-decay` | leaf | memory | High similarity (>0.8) to memory-type, episodic-memory; BuildContext connects to PRD |
| `semantic-memory` | leaf | memory | BuildContext returns 20+ relations including working-memory, episodic-memory |
| `retrieval-quality` | leaf | quality | High similarity (0.889) to search-quality |
| `mira` | leaf | agents | BuildContext returns 9 relations; unique entity with lower similarity scores |
| `wikilinks` | leaf | graph | Similarity (0.65) to wiki-links variant |
| `buildcontext` | leaf | tools | FindSimilar returns tool/architecture concepts (0.55-0.59) |
| `findsimilarconcepts` | leaf | tools | Similarity to search-quality (0.65), hybrid-search (0.62) |
| `azure-sql` | leaf | azure | High similarity (>0.78) to azure-sql-database variant |

### Control Queries (5)

| Query | Type | Expected Behavior |
|-------|------|-------------------|
| `xyzzy123garbage` | random string | All similarity scores < 0.62 |
| `aaa111bbb222ccc` | alphanumeric noise | All similarity scores < 0.62 |
| `q` | single character | All similarity scores < 0.62 |
| `the` | stopword | All similarity scores < 0.60 |
| `!@#$%^&*()` | special characters | Input validation error or very low scores |

---

## T-QUAL-GATE-002: Acceptance Thresholds

### BuildContext Metrics

| Metric | Threshold | Pass Condition | Fail Condition | Blocking? |
|--------|-----------|----------------|----------------|-----------|
| **Precision@10** | >= 0.70 | At least 7/10 top relations are semantically relevant | < 7/10 relevant | YES |
| **Hub Pollution Rate@10** | <= 0.20 | At most 2/10 relations are generic hubs for non-hub anchors | > 2/10 hubs | WARN |
| **Evidence Concentration** | <= 0.50 | No single file provides > 50% of evidence | > 50% from one file | WARN |
| **Preview Grounding Rate** | >= 0.90 | 9/10 previews contain anchor in first 200 chars | < 9/10 grounded | YES |

### FindSimilarConcepts Metrics

| Metric | Threshold | Pass Condition | Fail Condition | Blocking? |
|--------|-----------|----------------|----------------|-----------|
| **Similarity Sanity** | != 1.000 plateau | No query returns all top-10 at 1.000 | Any plateau at 1.000 | YES |
| **Score Variance** | >= 0.05 | Top-10 scores span at least 0.05 | Variance < 0.05 | YES |
| **Control Rejection** | max <= 0.62 | Garbage queries score <= 0.62 | Control >= 0.65 | YES |
| **Meaningful Match** | >= 0.65 | Valid concepts have 1+ result >= 0.65 | No match >= 0.65 | WARN |

---

## Release Gate Decision

| Outcome | Condition | Action |
|---------|-----------|--------|
| **PASS** | All BLOCKING pass AND all WARN pass | Release approved |
| **CONDITIONAL** | All BLOCKING pass, 1-2 WARN fail | Requires documented justification |
| **FAIL** | Any BLOCKING fails | Release blocked until fixed |

---

## Metric Definitions

| Metric | Definition |
|--------|------------|
| **Precision@10** | Human-judged relevance. Mark each relation "relevant" or "irrelevant". Precision = relevant / 10 |
| **Hub Pollution Rate@10** | Count of generic hub concepts (tool, agent, workflow, maenifold, mcp, azure) in top-10 for non-hub anchors |
| **Evidence Concentration** | Max proportion of evidence from any single file. If 8/10 relations cite one file, concentration = 0.80 |
| **Preview Grounding Rate** | Proportion of previews where anchor concept appears in first 200 characters |
| **Similarity Sanity** | Binary: any query returning all 1.000 scores indicates embedding collapse |
| **Score Variance** | max(top-10) - min(top-10). Healthy distribution shows differentiation |
| **Control Rejection** | Max similarity across garbage control queries |
| **Meaningful Match** | Minimum similarity (0.65) to consider a result meaningful vs noise |

---

## Running the Quality Gate

```bash
# 1. Run evaluation suite against current graph
# (Manual process - run each query and record results)

# BuildContext test
maenifold buildcontext --concept "maenifold" --depth 2 --maxEntities 20

# FindSimilarConcepts test
maenifold findsimilarconcepts --concept "maenifold" --maxResults 10

# Control test
maenifold findsimilarconcepts --concept "xyzzy123garbage" --maxResults 10
```

### Checklist

- [ ] Run all 12 test concepts through BuildContext
- [ ] Run all 12 test concepts through FindSimilarConcepts
- [ ] Run all 5 control queries through FindSimilarConcepts
- [ ] Score Precision@10 for BuildContext (sample 3 hub + 3 leaf)
- [ ] Verify no 1.000 plateau in FindSimilarConcepts
- [ ] Verify control rejection (max < 0.62)
- [ ] Document results in release notes

---

## PRD Traceability

| Metric | PRD Requirement |
|--------|-----------------|
| Precision@10, Hub Pollution, Evidence Concentration | FR-7.3 (BuildContext returns meaningful context) |
| Preview Grounding Rate | FR-7.3 (content previews are grounded) |
| Similarity Sanity, Score Variance | NFR-7.4.2 (similarity output bounded) |
| Control Rejection, Meaningful Match | FR-7.4 (meaningful rankings for valid inputs) |

---

## Empirical Baseline (2026-02-01)

| Observation | Value | Source |
|-------------|-------|--------|
| Graph size | 115 concepts, 2773 relations | memory_status |
| Control query max | 0.62 | FindSimilarConcepts on garbage |
| Valid concept range | 0.65-0.83 | FindSimilarConcepts on test set |
| Score variance (typical) | 0.07-0.15 | Top-10 spread |

This baseline was established after T-QUAL-FSC2 (plateau fix) was completed.
