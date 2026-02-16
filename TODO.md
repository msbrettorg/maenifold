# TODO

## Active backlog

- REL-001: Release v1.0.3 tasks (**Complete**)
- OPENCODE-001: OpenCode Plugin active work
- MAENIFOLDPY-001: Python port tasks
- DIST-001: Windows MSI installer
- CLEANUP-001: Active thinking sessions (**Complete**)
- COMMUNITY-001: Community detection (Louvain)
- SITE-001: Site rebuild ("Warm Restraint")

## Sprint: Session Abandonment Sweep (T-CLEANUP-001.1)

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-CLEANUP-001.1 | SWE: Implement DB-driven abandoned session sweep in ConceptSync pre-pass + SessionCleanup.TryMarkAbandonedFromLastIndexed | FR-14.6 | **Complete** |

## Sprint: Community Detection (T-COMMUNITY-001)

Wave 1 — Foundation (serial):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-COMMUNITY-001.1 | SWE: Add concept_communities table schema with FK to concepts (`GraphDatabase.cs`) | FR-13.3 | **Complete** |
| T-COMMUNITY-001.2 | SWE: Implement Louvain algorithm — in-memory, deterministic seed, configurable gamma (`CommunityDetection.cs` new) | FR-13.1, FR-13.2, FR-13.11 | **Complete** |
| T-COMMUNITY-001.3 | SWE: Add config — MAENIFOLD_LOUVAIN_GAMMA, sibling thresholds, watcher debounce (`Config.cs`) | FR-13.11, NFR-13.5.1, NFR-13.9.1 | **Complete** |
| T-COMMUNITY-001.4 | Blue-team: Unit tests for Louvain — deterministic seed, known topology, empty graph, disconnected components (11 tests) | FR-13.1, FR-13.2 | **Complete** |

Wave 2 — Integration (parallel after Wave 1):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-COMMUNITY-001.5 | SWE: Hook community detection into full Sync — run after concept extraction, persist results (`ConceptSync.cs`) | FR-13.4 | **Complete** |
| T-COMMUNITY-001.6 | SWE: DB file watcher with debounce for community recomputation — reuse FileSystemWatcher pattern, skip own writes (`IncrementalSync.CommunityWatcher.cs` new) | FR-13.5 | **Complete** |
| T-COMMUNITY-001.7 | SWE: Add community_id to RelatedConcept in BuildContext via indexed query (`GraphTools.cs`, `BuildContextResult.cs`) | FR-13.6 | **Complete** |
| T-COMMUNITY-001.8 | SWE: Add CommunitySiblings section to BuildContext — normalized weighted overlap, thresholds, cap 10 (`GraphTools.cs`, `BuildContextResult.cs`) | FR-13.7, FR-13.8, FR-13.9 | **Complete** |
| T-COMMUNITY-001.9 | SWE: Graceful degradation — omit community fields silently when no data (`GraphTools.cs`) | FR-13.10 | **Complete** |

Wave 3 — Verification:

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-COMMUNITY-001.10 | Blue-team: Integration tests — sync hook, BuildContext enrichment, siblings, degradation (6 tests) | FR-13.4–13.10 | **Complete** |
| T-COMMUNITY-001.11 | Red-team: Adversarial review — found FAIL-001 (non-atomic write + missing guard), remediated | Security | **Complete** |
| T-COMMUNITY-001.12 | Blue-team: NFR compliance — 12/12 PASS | NFR-13.x | **Complete** |

## Sprint: Site Rebuild (T-SITE-001)

Wave 1 — Foundation (serial dependency):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-SITE-001.1 | SWE: Delete all decorative components, animation CSS, `@headlessui/react`, use-case pages, stale routes | T-SITE-001.1 | Pending |
| T-SITE-001.2 | SWE: Implement design system foundation (CSS vars, palette, system fonts, feTurbulence noise, Tailwind v4 @theme) | T-SITE-001.2 | Pending |
| T-SITE-001.3 | SWE: Implement layout shell (dark default, theme cascade script, skip link) | T-SITE-001.3 | Pending |

Wave 2 — Components + Pages (7 parallel tasks after Wave 1):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-SITE-001.4 | SWE: Implement Header (flat nav, horizontal scroll mobile, no hamburger) | T-SITE-001.4 | Pending |
| T-SITE-001.5 | SWE: Implement Footer (logo, dynamic version, brand statement, MIT, GitHub) | T-SITE-001.5 | Pending |
| T-SITE-001.6 | SWE: Implement Home page (product desc, install, MCP config, CLI examples, platform table, graph screenshot) | T-SITE-001.6 | Pending |
| T-SITE-001.7 | SWE: Implement `/docs` page (render docs/README.md) | T-SITE-001.7 | Pending |
| T-SITE-001.8 | SWE: Implement `/plugins` page (render both plugin READMEs with Mermaid) | T-SITE-001.8 | Pending |
| T-SITE-001.9 | SWE: Implement `/tools` page (data-driven catalog from tools/*.md) | T-SITE-001.9 | Pending |
| T-SITE-001.10 | SWE: Implement `/workflows` page (data-driven catalog from workflows/*.json) | T-SITE-001.10 | Pending |

Wave 2 — Shared infrastructure (parallel with pages):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-SITE-001.11 | SWE: Implement Shiki syntax highlighting with custom warm-restraint theme | T-SITE-001.11 | Pending |
| T-SITE-001.12 | SWE: Implement build-time Mermaid rendering via mmdc with design system theme | T-SITE-001.12 | Pending |
| T-SITE-001.13 | SWE: Implement CopyButton component | T-SITE-001.13 | Pending |

Wave 3 — Verification:

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-SITE-001.14 | Blue-team: Lighthouse audit + keyboard nav verification | T-SITE-001.14 | Pending |
| T-SITE-001.15 | Red-team: Security audit (XSS, injection, build pipeline, deps) | T-SITE-001.15 | Pending |
| T-SITE-001.16 | Blue-team: Full FR-15.x compliance verification + content accuracy | T-SITE-001.16 | Pending |

## Sprint: Sync Mtime Optimization (T-SYNC-MTIME-001)

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-SYNC-MTIME-001.1 | Blue-team: add full Sync mtime/hash guard clause tests | TC-14.1-14.4 | **Complete** |
| T-SYNC-MTIME-001.2 | SWE: implement full Sync mtime → hash → process chain (no pre-read, independent guards) | FR-14.1-14.4 | **Complete** |
| T-SYNC-MTIME-001.3 | Blue-team: add incremental Sync mtime/hash guard clause tests | TC-14.5 | **Complete** |
| T-SYNC-MTIME-001.4 | SWE: implement incremental Sync mtime → hash → process chain (store filesystem mtime in last_indexed; maintain file_md5) | FR-14.5 | **Complete** |
| T-SYNC-MTIME-001.5 | Red-team audit | Security | **Complete** |
| T-SYNC-MTIME-001.6 | Blue-team verify coverage held under attack | Security | **Complete** |

## Sprint: WikiLink Write Filter & Hub Detection (T-WLFILTER-001)

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-WLFILTER-001.1 | Implement WikiLinkFilter.cs filter engine | FR-11.1 | **Complete** |
| T-WLFILTER-001.2 | Create memory-hub-detection.json workflow | FR-11.3 | **Complete** |
| T-WLFILTER-001.3 | Integrate WikiLinkFilter into WriteMemory | FR-11.2 | **Complete** |
| T-WLFILTER-001.4 | Integrate WikiLinkFilter into EditMemory | FR-11.2 | **Complete** |
| T-WLFILTER-001.5 | Add hub-detection as 5th specialist to memory-cycle.json | FR-11.4 | **Complete** |
| T-WLFILTER-001.6 | Write WikiLinkFilter unit tests | TC-11.1 | **Complete** |
| T-WLFILTER-001.7 | Write WriteMemory/EditMemory filter integration tests | TC-11.2-4 | **Complete** |
| T-WLFILTER-001.8 | Red-team audit | Security | **Complete** |

## Sprint: OpenCode Plugin Integration (T-OC-PLUGIN-001)

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-OC-PLUGIN-001.1 | Implement FLARE session start context injection (`experimental.chat.system.transform`) | FR-12.1 | Pending |
| T-OC-PLUGIN-001.2 | Implement Task augmentation with WikiLink graph context (`tool.execute.before`) | FR-12.2 | Pending |
| T-OC-PLUGIN-001.3 | Implement WikiLink tagging guidelines injection into compaction (`experimental.session.compacting`) | FR-12.3 | Pending |
| T-OC-PLUGIN-001.4 | Implement concept/decision extraction + WriteMemory persistence during compaction | FR-12.4 | Pending |
| T-OC-PLUGIN-001.5 | Implement SequentialThinking compaction persistence (`event` on `session.compacted`) | FR-12.5 | Pending |
| T-OC-PLUGIN-001.6 | Implement ConfessionReport enforcement (`tool.execute.after` on `task`) | FR-12.6 | Pending |
| T-OC-PLUGIN-001.7 | Implement unified plugin scaffold: CLI discovery, timeouts, graceful degradation, logging | FR-12.7 | Pending |
| T-OC-PLUGIN-001.8 | Red-team audit | Security | Pending |

---

_See `RETROSPECTIVES.md` for completed work and historical notes._
