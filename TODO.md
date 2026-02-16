# TODO

## Active backlog

- REL-001: Release v1.0.3 tasks (**Complete**)
- OPENCODE-001: OpenCode Plugin active work
- MAENIFOLDPY-001: Python port tasks
- DIST-001: Windows MSI installer
- CLEANUP-001: Active thinking sessions
- SITE-001: Site rebuild ("Warm Restraint")

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
| T-SYNC-MTIME-001.1 | Blue-team: add full Sync mtime/hash guard clause tests | TC-14.1-14.4 | Pending |
| T-SYNC-MTIME-001.2 | SWE: implement full Sync mtime → hash → process chain (no pre-read, independent guards) | FR-14.1-14.4 | Pending |
| T-SYNC-MTIME-001.3 | Blue-team: add incremental Sync mtime/hash guard clause tests | TC-14.5 | Pending |
| T-SYNC-MTIME-001.4 | SWE: implement incremental Sync mtime → hash → process chain (store filesystem mtime in last_indexed; maintain file_md5) | FR-14.5 | Pending |
| T-SYNC-MTIME-001.5 | Red-team audit | Security | Pending |
| T-SYNC-MTIME-001.6 | Blue-team verify coverage held under attack | Security | Pending |

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
