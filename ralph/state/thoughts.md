# RALPH Thoughts

Handoff from iter-6 (2026-05-16, ~45 min). **All non-UAT non-blocked backlog drained.** Iter-6 shipped 19 task ships + 3 sprint umbrella closures = 22 things, zero stuck worktrees, zero cascade incidents, T-325.3 expectedStage enforcement validated in production.

## ⚠️ NEXT-PM FIRST READ — open items requiring carve-outs

1. **T-309.3** (sprint 309, P1, blocked) — fs/write permission renderer-trust hardening. Correctly blocked per AGENTS.md P0 trigger #5 (anti-cargo-cult-security). Unblocks ONLY on real XSS / supply-chain incident, DOMPurify bypass, user explicit authorization, or external review requiring it. **Do NOT dispatch.**

2. **task-pm terminal-stage RACI drift (UNRESOLVED process issue)** — 3 iter-6 task-pms (T-322.24, T-322.1.4, T-324.5) self-passed the terminal stage instead of stopping at the last intermediate. All self-reported the violation honestly and the work was independently verified clean, so iter-6 accepted as-shipped. Mitigation iter-6 was a "RACI reminder" prompt block on subsequent dispatches (none drifted after the addition). **Permanent fix candidates** (filed in `ralph/state/learnings.md` iter-6 entry, NOT yet authored as a spec): (A) `~/.copilot/agents/task-pm.md` global update with the same reminder block hard-coded, (B) server-side enforcement analog of T-325.3 — last-intermediate → done requires agent identity matching `ralph-pm`, (C) accept as-is. Iter-7 PM should decide whether to file a new sprint-326 task and ship the fix.

## TL;DR — iter-6 result

**Massive iter — all known-bug fixes drained.** 19 task ships + 3 sprint umbrella closures (T-322.0, T-324.0, T-325.0). 2 WONTFIX-on-cited-evidence cascades (T-303.10, T-303.4.21). 13 sprint-322 children shipped. 1 sprint-324 child shipped (T-324.5 closing the sprint). 1 sprint-325 child shipped (T-325.2 closing the sprint). 2 new tasks discovered + filed + dispatched + shipped same iter (T-322.1.5, T-322.1.6 from T-322.1.4 UX research — perfect NO PUNTING execution). 1 direct-dispatch SWE remediation (T-322.16 test async-timing, 112s mechanical fix). 0 cascade incidents. 0 user interruptions. 0 worktrees at iter close.

## What shipped this iter — full ledger

**SWE merges to dev (15 task shipments):**

| Task | Sprint | Scope | Diff |
|------|--------|-------|------|
| T-322.31 | 322 | Vitest conversion for check-node-modules-integrity-missing.test.mjs | 1 file, +14/-14 |
| T-322.32 | 322 | deploy-daemon.sh lsof sanity check (exit 4 on unexpected path) | 1 file, +5 |
| T-322.17 | 322 | useResizeHandle → VueUse useEventListener | 1 file, +9/-8 |
| T-322.18 | 322 | pipeline-mcp-polling → VueUse useIntervalFn (3-failure-stop preserved) | 1 file, +5/-4 |
| T-322.24 | 322 | ToolNodeFields wrapper deleted; ToolNodeFieldsTool used directly | 3 files, +9/-48 |
| T-322.34 | 322 | Edge-case enumeration rule refinements (9 1-line additive clarifications) | 3 files, +19/-8 |
| T-322.30 | 322 | sanitizeConceptForWikilinkQuery applied to 2 cross-stack callers + 9 tests | 5 files, +59 |
| T-322.33 | 322 | deploy-daemon.sh PID-discovery retry loop (DEPLOY_DAEMON_PID_RETRY_SECONDS) | 1 file, +14/-4 |
| T-322.16 | 322 | local-storage.ts deleted; 4 production files migrated to VueUse useLocalStorage; tests updated for async-write timing | 14 files, +156/-250 |
| T-324.5 | 324 | docs/DEVELOPMENT.md env-source reference (4 shapes + precedence + spawn-paths + example snippet) | 1 file, +86/-23 |
| T-322.6 | 322 | memory:// hover preview cards in MarkdownView (reka-ui HoverCard, 300ms delay, T-322.4 cache reuse) | 5 files, +213/-181 (1 modified + 4 new) |
| T-325.2 | 325 | gate_Reopen accepts done-located tasks + atomic mv done→todo + slug recovery + 3 new TDD tests | 3 files, +113/-31 |
| T-322.1.4 | 322 | gsd UX research for list-surface sort decisions; 2 child tasks filed + dispatched same iter | (no code) |
| T-322.1.5 | 322 | RunHistoryPanel sort dropdown (3 options, session persistence) — discovered + shipped iter-6 | 2 files, +80/-6 |
| T-322.1.6 | 322 | KnowledgeVaultBrowser sort controls (tree+search, folders-first, per-mode persistence) — discovered + shipped iter-6 | 2 files, +354/-23 (1 modified + 1 new test) |

**WONTFIX-on-cited-evidence cascades (2):**
- T-303.10 (UAT for non-existent test MCP app — verified via apps/desktop/docs/mcp-test-apps.md)
- T-303.4.21 (multi-turn sampling UI UAT — fixture would require fake or external-LLM, both forbidden)

**Sprint umbrella closures (3):**
- T-322.0 — sprint 322 misc backlog (declared child T-322.1 done + 42 sub-task ships)
- T-324.0 — sprint 324 config env passthrough portability (T-324.1/.2/.3/.5 all done; goal achieved)
- T-325.0 — sprint 325 gate tooling improvements (T-325.1/.2/.3 all done; cascade incident class permanently fixed)

**PM-direct shipments (1):**
- `ralph/state/learnings.md` +24 lines documenting task-pm terminal-stage RACI drift

## What's at the TOP of the iter-7 dispatch queue

**Critical:** No non-UAT non-blocked work remains in `todo/`. Iter-7 PM options:

1. **Wait for user direction** — backlog is fully drained of bug-fix work. Iter-7 PM should ask the user explicitly: "All known-bug work is shipped. Do you want a UAT pass (which would dispatch the 11 UAT-floor items via CDP), close the stale 304/311/316 UAT umbrellas as WONTFIX, file new feature work, or work on the task-pm RACI drift permanent fix?" Per AGENTS.md no-punting rule, asking-as-only-option requires the question to be plain English with numbered choices.

2. **WONTFIX cascade stale UAT umbrellas** — sprint 304 umbrella + 4 UAT children; sprint 311 umbrella + 1 UAT child; sprint 316 umbrella + 1 UAT child. Many of these UAT tasks reference behavior that has changed since their authoring (likely Q1 2026). Iter-7 PM could verify per task body whether the target surface still exists in current code; cascade WONTFIX on any whose premise has decayed.

3. **Task-pm RACI drift permanent fix** — file as T-326.1 (start new sprint 326 for process improvements). Either update `~/.copilot/agents/task-pm.md` with explicit WorkResult skeleton + RACI-reminder hardcoded, OR add server-side enforcement to gate-core. Lesson + permanent fix candidates already documented in `ralph/state/learnings.md` iter-6 entry.

4. **User-requested UAT pass** — if user explicitly asks. Each UAT task uses `scripts/uat-launch.sh <task-id>` for isolated CDP. agent-browser only (NO Python/Playwright per AGENTS.md UI Verification). One UAT task per task-pm dispatch.

## Remaining backlog snapshot (16 items, ALL UAT-floor or constitution-blocked)

- Sprint 303 UAT (4): T-303.9, T-303.18.2, T-303.18.6, T-303.19.1 — older UAT carry-overs
- Sprint 304 + 4 UAT children: T-304.0 umbrella, T-304.2 reconnect, T-304.3 agents-perf (functionally-UAT CDP measurement), T-304.6 rendering matrix, T-304.19 cdp-uat-16/17
- Sprint 305 UAT (1): T-305.17 clear-history-test-refresh
- Sprint 309 constitution-blocked (1): T-309.3 write-permission-renderer-trust-hardening
- Sprint 311 (1 UAT + umbrella): T-311.0, T-311.2 visual-explainer-skill-uat
- Sprint 316 (1 UAT-ish + umbrella): T-316.0, T-316.2 plyr-av-player-evaluation

NONE of these are dispatchable autonomously by the PM. UAT requires CDP + user-confirmed scope (per UAT-priority-floor rule).

## Daemon / process state at iter close

- Daemon PID 43641 (started 12:48PM 2026-05-16) on iter-5 binary at iter-6 start; T-325.3 EXPECTED_STAGE_REQUIRED enforcement verified live (binary grep matched + caught a real ralph-pm misstep mid-iter).
- HEAD: 9a595af49 = "Merge T-322.1.6: KnowledgeVaultBrowser sort controls". 29 commits added iter-6.
- **Zero worktrees in flight at iter close** — all 12 task-pms dispatched this iter returned + were shipped + cleaned up.
- One T-325.2 changed crates/maenifold-gate-core — daemon redeploy recommended if iter-7 PM intends to use the new gate_Reopen done→todo path heavily. Otherwise the iter-5 binary continues working (it has T-325.3 enforcement but lacks the T-325.2 reopen extension).

## Process improvements landed iter-6 (encoded in repo or docs)

1. `ralph/state/learnings.md` appended with **task-pm terminal-stage RACI drift** lesson + permanent fix candidates (commit 18d5af082).
2. T-325.3 expectedStage enforcement validated end-to-end in production (caught real cascade attempt + 0 cascade incidents across 12 task-pms).
3. "RACI reminder" prompt block added to all dispatch prompts after T-322.24 drift detected — subsequent task-pms (T-322.33, T-325.2, T-322.1.5, T-322.1.6) did not drift.
4. Same-file contention handling pattern documented: T-322.33 worktree recreated AFTER T-322.32 merged so the worker saw current dev.

## Open process issues for iter-7

1. **task-pm RACI drift permanent fix** — see "NEXT-PM FIRST READ" item #2. Recommend filing as T-326.1 and shipping iter-7.
2. **Stale UAT umbrellas** — sprint 304/311/316 UAT umbrellas may have decayed premises. Recommend per-task premise verification before any UAT pass.
3. **Wave-boundary rubber-duck reviews** — none triggered iter-6 (no cross-cutting umbrella close requiring inter-dependent verification). Iter-7 may need them if filing a new structural sprint (e.g., the task-pm RACI permanent fix).

## Iter-6 throughput tally — for the record

- **15 SWE shipments merged to dev** (12 sprint 322 + 1 sprint 324 + 1 sprint 325 + 1 from research-only T-322.1.4)
- **3 sprint umbrella closures** (T-322.0, T-324.0, T-325.0) — sprint 322 fully drained, sprint 324 + 325 sprint goals achieved
- **2 WONTFIX cascades** with cited evidence (T-303.10, T-303.4.21)
- **2 new tasks filed + dispatched + shipped same iter** (T-322.1.5, T-322.1.6) — perfect NO PUNTING execution
- **1 PM-direct shipment** (`ralph/state/learnings.md` task-pm RACI drift lesson)
- **1 direct-dispatch SWE remediation** (T-322.16 test async-timing fix, 112s mechanical)
- **0 cascade incidents** (T-325.3 expectedStage enforcement validated in production)
- **0 worktrees left at iter close** — clean handoff
- **0 user interruptions during dispatch**
- **3 task-pm RACI-drift instances** — all self-reported honestly, all work verified clean independently, mitigation applied mid-iter; permanent fix deferred to iter-7
- **29 git commits across iter-6** (15 worker commits + 14 PM merge commits)
- **Wall-clock: ~45 minutes** for the full iter
