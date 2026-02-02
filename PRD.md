# Product Requirements Document: Maenifold

## Document Control

| Version | Date | Author | Description |
|---------|------|--------|-------------|
| 1.0 | 2026-01-31 | PM Agent | Net-new PRD for T-QUAL-FSC2 (FindSimilarConcepts embedding quality) |
| 1.1 | 2026-02-01 | PM Agent | Added FR-7.6 (access boosting), tiered decay, NFR-7.5.3-7.5.6 per Moltbook research |
| 1.2 | 2026-02-01 | PM Agent | Added FR-7.7 (ListMemories decay metadata) |
| 1.3 | 2026-02-01 | PM Agent | Added FR-7.8 (assumption decay by status) |
| 1.4 | 2026-02-01 | PM Agent | Refined thinking tier: sequential=7d, workflows=14d per Moltbook gap analysis |
| 1.5 | 2026-02-01 | PM Agent | **P0**: Added FR-8.x (CLI JSON output) - blocks OpenClaw integrations |
| 1.6 | 2026-02-01 | PM Agent | Added FR-9.1 (retrospectives log) |
| 1.7 | 2026-02-01 | PM Agent | Added FR-9.2 (debug-only build/test requirement) |

---

## 1. Executive Summary

**Product Area**: Maenifold graph embeddings and semantic similarity
**Objective**: Eliminate similarity plateaus caused by embedding collapse, ensuring FindSimilarConcepts returns meaningful rankings for valid inputs.

---

## 2. Problem Statement

Current `FindSimilarConcepts` results can degenerate into a similarity plateau (many unrelated concepts at 1.000 similarity) when embeddings collapse to identical vectors. This undermines semantic search quality and user trust.

---

## 3. Functional Requirements

### 3.1 CLI JSON Output (P0 - Highest Priority)

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-8.1 | CLI SHALL support a `--json` flag that outputs structured JSON instead of human-readable markdown. | **P0** |
| FR-8.2 | All MCP tools SHALL return structured JSON when `--json` flag is present. | **P0** |
| FR-8.3 | JSON output SHALL include `success`, `data`, and `error` fields for consistent parsing. | **P0** |
| FR-8.4 | Error responses SHALL include structured error codes and human-readable messages. | **P0** |
| FR-8.5 | Omitting `--json` flag SHALL return current markdown format (backward compatible). | **P0** |

**Rationale**: OpenClaw integrations (SessionChannel, Memory Plugin) currently parse CLI output using fragile regex on markdown. Structured JSON enables reliable programmatic access and blocks critical integration work.

**Blocking**:
- OpenClaw SessionChannel JSON migration (eliminates regex parsing)
- OpenClaw Memory Plugin (maenifold as memory provider)

### 3.2 Embeddings Quality

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-7.4 | System SHALL discover semantically similar concepts via embeddings and return meaningful rankings for valid inputs. | P1 |
| FR-7.5 | System SHALL apply recency-based decay weighting to all search result rankings with tiered grace periods and configurable half-life. | P1 |
| FR-7.6 | System SHALL reset decay clock (update `last_accessed` timestamp) when a memory is explicitly read via ReadMemory, boosting frequently-accessed content. | P1 |
| FR-7.7 | ListMemories SHALL display decay-relevant metadata (`created`, `last_accessed`, `decay_weight`) for each file. | P2 |
| FR-7.8 | AssumptionLedger assumptions SHALL decay based on status: `validated` assumptions are exempt; `active`, `refined`, and `invalidated` assumptions decay. | P1 |

### 3.3 Product Governance

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-9.1 | Repository SHALL include `RETROSPECTIVES.md` to capture sprint retrospectives. | P2 |
| FR-9.2 | During active sprints, builds and tests SHALL use Debug configuration only; Release build SHALL NOT be invoked. | P2 |

---

## 4. Non-Functional Requirements

### 4.1 CLI JSON Output

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-8.1.1 | JSON output SHALL be valid UTF-8 encoded JSON parseable by standard libraries. | Required |
| NFR-8.1.2 | JSON output SHALL complete within the same time bounds as markdown output (no performance regression). | Required |
| NFR-8.1.3 | JSON error responses SHALL include error code, message, and optional details object. | Required |
| NFR-8.1.4 | JSON output SHALL NOT include ANSI escape codes or terminal formatting. | Required |

### 4.2 Embeddings Quality

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-7.4.1 | Tokenization MUST be compatible with the embedding model's vocab and IDs; mismatches SHALL fail closed. | Required |
| NFR-7.4.2 | Similarity output SHALL NOT exceed 1.000. | Required |
| NFR-7.5.1 | Default decay grace period for `memory/thinking/sequential/` SHALL be 7 days with env override (`MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL`). | Required |
| NFR-7.5.1a | Default decay grace period for `memory/thinking/workflows/` SHALL be 14 days with env override (`MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS`). | Required |
| NFR-7.5.2 | Default decay grace period for all other memory SHALL be 14 days with env override (`MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT`). | Required |
| NFR-7.5.3 | Default decay half-life SHALL be 30 days with env override (`MAENIFOLD_DECAY_HALF_LIFE_DAYS`). | Required |
| NFR-7.5.4 | Decay SHALL affect search ranking only; decayed content SHALL remain fully retrievable via direct query (no deletion). | Required |
| NFR-7.6.1 | ReadMemory SHALL update the file's `last_accessed` timestamp on every read. | Required |
| NFR-7.6.2 | SearchMemories SHALL NOT update `last_accessed` for files appearing in results. | Required |
| NFR-7.6.3 | BuildContext SHALL NOT update `last_accessed` for files referenced in context. | Required |
| NFR-7.7.1 | ListMemories decay_weight SHALL be computed using the file's tier (sequential=7d, workflows=14d, other=14d grace) and current age. | Required |
| NFR-7.8.1 | `validated` assumptions SHALL NOT decay (exempt from decay weighting). | Required |
| NFR-7.8.2 | `active` and `refined` assumptions SHALL use 14-day grace period and 30-day half-life. | Required |
| NFR-7.8.3 | `invalidated` assumptions SHALL use 7-day grace period and 14-day half-life (aggressive decay). | Required |

---

## 5. Design Notes

### Permanent Knowledge (No Decay)

Content that should never decay is stored outside the `memory://` system:

| Content Type | Location | Rationale |
|--------------|----------|-----------|
| Procedural knowledge | Skills, system prompts | Injected at session start; not subject to retrieval ranking |
| Core identity / values | AGENTS.md, system prompts | Session context, not searched |
| Architecture decisions | Code, docs, AGENTS.md | Source of truth is the repo itself |
| Security lessons | AGENTS.md, code comments | Critical guidance lives with the code it protects |

This separation means `memory://` only contains knowledge that *should* decay — ephemeral session thinking and evolving notes. Permanent knowledge doesn't need a "pinned" flag because it's architecturally separate.

### Thinking Tiers (NFR-7.5.1, NFR-7.5.1a)

Thinking content is tiered by type, not treated uniformly:

| Path | Grace Period | Rationale |
|------|--------------|-----------|
| `thinking/sequential/` | 7 days | Episodic reasoning tied to specific tasks; valuable while task is active |
| `thinking/workflows/` | 14 days | Structured multi-step processes; longer-lived than single reasoning sessions |

**Principle**: SequentialThinking sessions are task-bound episodic memory (what reasoning happened during a specific investigation). Workflows are closer to procedural artifacts (how to accomplish multi-step processes). Different cognitive types warrant different decay curves, per Moltbook community research.

### Access Boosting Scope (FR-7.6)

Only **ReadMemory** updates `last_accessed`. SearchMemories and BuildContext do not.

| Tool | Updates `last_accessed`? | Rationale |
|------|--------------------------|-----------|
| ReadMemory | ✅ Yes | Explicit, intentional access — user deliberately requested this content |
| SearchMemories | ❌ No | Appearing in search results ≠ being read; would boost 10 files when user reads 1 |
| BuildContext | ❌ No | Automated context enrichment runs frequently; would grant accidental immortality to hub concepts |

**Principle**: Access boosting rewards *deliberate use*, not *passive appearance*. A file should only reset its decay clock when someone actually reads it, not when it shows up in a list or gets pulled into context automatically.

### Assumption Decay by Status (FR-7.8)

Assumptions decay based on their epistemic status, creating pressure to validate or discard:

| Status | Decay Behavior | Rationale |
|--------|----------------|-----------|
| `validated` | ❌ No decay | Confirmed knowledge; treat as permanent |
| `active` | 14d grace, 30d half-life | Untested hypotheses should surface pressure to validate |
| `refined` | 14d grace, 30d half-life | Superseded by newer version; original should fade |
| `invalidated` | 7d grace, 14d half-life | Historical record only; aggressive decay to deprioritize |

**Principle**: Epistemic hygiene through decay. Active assumptions that linger without validation naturally lose retrieval priority, creating implicit pressure to either validate them (granting immortality) or let them fade (implicit invalidation).

---

## 6. Out of Scope

- Changing the embedding model itself.
- Introducing new ranking algorithms beyond cosine similarity.
- Per-file decay rate configuration (all files in a tier use the same rate).
- Explicit "pinned" or "no-decay" flags within `memory://` (use external sources instead).
