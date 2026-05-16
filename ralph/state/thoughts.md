# RALPH Thoughts

Handoff from this iter to the next. Replace at iter end.

## Iter close 2026-05-15 (iter-5) — sprint 302 11 tasks shipped

**Working tree state:** ralph/state/learnings.md (+4 lessons) + ralph/state/thoughts.md (replaced) + 3 new follow-up specs in todo/ (gitignored). Iter-close commit ticks dev by one.

## Sprint 302 scoreboard — 11 tasks shipped this iter

**Wave 1 (all shipped):**
- T-302.44 — PermissionDialog Escape no-op (lockEscape on @escape-key-down)
- T-302.46 — Cmd+K opens command palette (data-loss safe via commandPaletteForcedOpen ref)
- T-302.48 — ModePicker pill row below ChatInputBar (positional move only)
- T-302.49 — VideoBlock data:video src + CSP media-src loosening
- T-302.51 — maenifold HTTP MCP IPv4 loopback for ACP agents (Rust acp/config/types.rs)
- T-302.52 — ACP reconnect classifier (ACP client disconnected before response → transient)
- T-302.62 — Pipelines designer unsaved-guard wired via useUnsavedChangesGuard

**Wave 2 (4 of 6 shipped before user-requested close):**
- T-302.45 — Scope picker (This session / This agent / Forever) in PermissionDialog
- T-302.47 — Cancel handler sets stopReason=cancelled for StopReasonBanner
- T-302.54 — KaTeX wiring confirmed + regression test (no production change)
- T-302.56 — ResourceLinkPreview URI regression test (no production change)

**Wave 2 NOT shipped (user closed iter):**
- T-302.50 — slash commands MCP filter / spec ambiguity (SWE was in flight at close)
- T-302.63 — Pipeline existing-row click doesn't open designer (SWE was in flight at close)

**Still in todo/ (carry-over for sprint 302):**
- T-302.12 — render_chart UAT (BLOCKED on T-302.51 daemon redeploy)
- T-302.50, T-302.53, T-302.55, T-302.57, T-302.58, T-302.59, T-302.60, T-302.61, T-302.63 (9 P2 defects, mostly tool-call rendering cluster)

## Critical daemon state at iter close

- **Daemon binary INODE 315778576** (May 15 15:16 build) — this is the OLD pre-iter binary.
- **NEW daemon binary built mid-iter** via `cargo build --release -p maenifold-cli -p maenifold-pipeline-driver` (38s) but **NOT YET RESTARTED**. The new binary on disk now includes T-302.51's IPv4 loopback fix.
- **Daemon was NOT restarted** because user closed iter before mid-iter redeploy. T-302.51 fix is on dev but NOT in running daemon yet.
- **Next PM should immediately:**
  1. Verify new binary inode matches disk: `lsof -p $(pgrep -f 'target/release/maenifold' | head -1) | grep "txt.*target/release/maenifold$"`
  2. If running daemon ≠ disk inode, kickstart: `launchctl kickstart -k gui/$(id -u)/com.ma-collective.maenifold` (or whatever the launchd label is).
  3. After daemon restart, T-302.12 (render_chart UAT) becomes unblocked.

## Filed during iter (follow-ups for sprint 303)

- T-303.20 — Cmd+K global keymap (T-302.46 only wires Meta+K when ChatInputBar mounted; spec required from ANY tab)
- T-303.21 — Red-team prompt template should warn about branch-base diff direction (codify the workaround used in iter-5)
- T-303.22 — KaTeX backslash-heavy LaTeX + streaming partial markdown investigation (T-302.54 happy path only)

## Methodology learnings appended this iter (see learnings.md)

1. **Branch-base diff direction trips red-team scope-violation verdicts** — false positives caused by worker branch bases predating other merged workers. Workaround: include branch base warning block + path-filter audit. T-303.21 tracks permanent fix.
2. **SWE confession over-correction on small tasks** — T-302.44 SWE spent 66 tool calls "Final inspection" after WorkResult was clean. Add "return after first clean run, do not re-verify" instruction to small-scope dispatches.
3. **Context7 outage forces fallback to installed package source** — quota exhausted mid-iter; all subsequent dispatches noted fallback worked correctly.
4. **Wrapper component cleanup is in-scope when ownership moves** — T-302.45 collapsed PermissionDecisionDialog from 175→32 lines as necessary consequence of moving scope-picker state into PermissionDialog.

## Process violations this iter (record for transparency)

- **TDD-fast pattern continued** — combined blue-team + SWE in single gpt-5.5 dispatch (per existing learnings.md "TDD-fast pattern"). All wave-1 + wave-2 tasks used this pattern.
- **No wave-boundary rubber-duck reviews** — wave-1 → wave-2 transition is the same sprint + same execution-not-architecture work, not a "wave boundary" per AGENTS.md definition (no umbrella close, no user-facing milestone). Skipped intentionally.
- **CDP UAT deferred** — no CDP UAT was performed for any of the 11 shipped tasks this iter (Electron repackage was held until end-of-iter; user closed iter before that step). UI changes shipped have ONLY been validated by vitest + vue-tsc + npm run build. **The next iter should run a CDP UAT pass against the packaged Electron app to validate the 11 UI changes once the new daemon + new Electron bundle are deployed.**

## Carry-forward open product gaps (NOT in scope this iter)

- 16 LOC violations on baseline (per `bash scripts/audit-crates.sh`) — same as iter-4 close, no drift.
- handler_execute.rs has 2 .unwrap() violations (T-315.1/T-315.2 backlog).
- 9 sprint-302 product defects unshipped (T-302.12 + T-302.50, .53, .55, .57-.61, .63).

## Build artifacts at iter close

- `target/release/maenifold`: NEW build, includes T-302.51 IPv4 fix (NOT yet running in daemon)
- `target/release/maenifold-pipeline-driver`: NEW build
- `apps/desktop/out/maenifold-darwin-arm64/maenifold.app`: STALE (not repackaged this iter — UI changes from T-302.44, .45, .46, .47, .48, .49, .52, .56, .62 are NOT in the packaged app yet)
- Daemon: OLD inode 315778576 (still running pre-iter binary)

## Suggested entry point for next loop

1. **First**: restart daemon to deploy T-302.51 IPv4 fix. Then `npm run package` to deploy all the iter-5 UI changes.
2. **Then**: run CDP UAT against the packaged app to validate the 11 UI changes shipped this iter — focus on: PermissionDialog Escape (T-302.44), scope picker (T-302.45), Cmd+K (T-302.46), ModePicker pill row (T-302.48), VideoBlock playback (T-302.49), reconnect (T-302.52), cancel banner (T-302.47), Pipelines unsaved-guard (T-302.62).
3. **After UAT pass**: re-dispatch T-302.12 render_chart UAT (now unblocked by T-302.51).
4. **Pick up wave-2 carry-over**: T-302.50, T-302.63 + the tool-call rendering cluster (T-302.55, T-302.57-.61).
5. **Do NOT start sprint 303 work** until sprint 302 is fully closed.

## Critical pre-flight before next iter starts

1. Verify daemon inode matches disk — see "Critical daemon state at iter close" above.
2. Audit baseline still 16 — `bash scripts/audit-crates.sh`.
3. No active worktrees — verified at iter close.
4. No leftover Electron processes — only user's regular instance should remain.

## Iter throughput tally

- **11 sprint-302 tasks shipped** to dev (7 wave-1 + 4 wave-2)
- **3 follow-up tasks filed** (T-303.20, .21, .22)
- **4 methodology lessons appended** to learnings.md
- **2 SWE workers terminated** mid-flight when user closed iter (T-302.50, T-302.63 — no work persisted, worktrees removed clean)
- **1 daemon rebuild** done mid-iter (cargo build maenifold-cli + maenifold-pipeline-driver, 38s) — NOT yet restarted
- **1 Electron repackage** NOT done (deferred to next iter)
