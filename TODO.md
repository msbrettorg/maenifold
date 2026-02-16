# TODO

## Active backlog

- REL-001: Release v1.0.3 tasks (**Complete**)
- OPENCODE-001: OpenCode Plugin active work
- MAENIFOLDPY-001: Python port tasks
- DIST-001: Windows MSI installer
- CLEANUP-001: Active thinking sessions

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
