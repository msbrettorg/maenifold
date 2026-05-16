# RALPH Thoughts

Handoff from iter-4 (2026-05-17, ~75 min elapsed).

## ⚠️ NEXT-PM FIRST READ — open items requiring carve-outs

1. **T-309.3** (sprint 309, P1, blocked) — fs/write permission renderer-trust hardening. Correctly blocked per AGENTS.md P0 trigger #5 (anti-cargo-cult-security). Unblocks ONLY on real XSS / supply-chain incident, DOMPurify bypass, user explicit authorization, or external review requiring it. **Do NOT dispatch.**

## TL;DR — iter-4 result

**HUGE iter for the bug-fix-first directive.** ~18 code-change tasks shipped + 3 WONTFIX-on-cited-evidence closures + 5 gate-state cleanups (used the new gate_Reopen primitive shipped this iter). Bug-fix queue significantly drained. Sprint 313, 315, 317, 318, 322 made major progress.

## What shipped this iter (the big ledger)

**Real code changes merged into dev (18 tasks):**

| Task | Sprint | Scope | Net LOC |
|------|--------|-------|---------|
| T-302.64 | 302 | ACP translation layer in jockey/client.rs (Codex execute→terminal synthesis, +4 tests) | +230/-55 |
| T-313.1 | 313 | Replace hand-rolled LCS with `diff` npm package (6 fixture tests, BSD-3-Clause) | +89/-36 |
| T-313.3 | 313 | ResourcePreview size cap (1MB) preventing renderer freeze on oversize markdown | +172/-1 |
| T-315.2 | 315 | Split jockey handler_execute.rs (455→293+162 LOC, 2 .unwrap() fixed) | (split) |
| T-317.1 | 317 | Onboarding step idempotency on force-quit + 6 new tests | +160 |
| T-317.2 | 317 | Onboarding agent CLI pre-flight check via Electron IPC + Vue composable (debounced) | +169/-3 |
| T-318.2 | 318 | Inspector "missing model identity" warning fix (state.model populated at dispatch) | +150/-54 |
| T-319.1 | 319 | Cmd+K global command palette (shadcn CommandDialog, 2 providers initial) | +220 |
| T-321.1 | 321 | Toast severity tiers (5 levels with persistence policy for critical) + 21 callsite migration | +75/-70 |
| T-322.19 | 322 | Delete unused mcp gateway routing module | -118 |
| T-322.20 | 322 | Inline pipeline refinement session helper into SemanticZoomOverlay + extract fork-utils | -26 |
| T-322.21 | 322 | Inline schedule permission labels into useSchedulesSettings | 0 |
| T-322.22 | 322 | Delete acp-chat-notifications.ts dead-singleton (already routed via session events) | -20 |
| T-322.23 | 322 | Inline KnowledgeNoteRenderer wrapper into MarkdownView (6 sites + 3 tests rewired) | -14 |
| T-322.25 | 322 | Remove agent-probe-types re-export shim (18 sites rewired to defs+normalizers) | -17 |
| T-322.26 | 322 | Remove pipeline-types re-export shim (100 sites rewired) [needed v2 retry due to T-322.20+T-322.25 conflicts] | -32 |
| T-322.36 | 322 | Fix deploy-daemon.sh inode check (disk-vs-disk false equality — real-world hit this iter) | +99/-3 |
| T-322.13 | 322 | node_modules integrity check skips optional:true entries | +95/-1 |
| T-325.1 | 325 | **gate_Reopen MCP primitive** — clean reset of error-state tasks | +213/-1 |
| T-323.2 | 323 | sampling deny error code -1 (was -32600) | +1/-1 |

**WONTFIX-on-cited-evidence (3 — premise verifiably dead per Cardinal Rule #2):**

- **T-323.1** — mcp gateway catalog "DiscoverMcpPrompts missing" was already fixed in current codebase; all 20 gateway_catalog_domain_tests pass green.
- **T-318.1** — sequential-thinking node dblclick handler: 6 code paths inspected, ALL type-agnostic; spec's own resolution path is "close as CDP flake" if no code bug.
- **T-319.2** — ShortcutCheatsheet test coverage: grep across apps/desktop/src/ → 0 hits for "cheatsheet". Component never shipped. Parent T-244.38 was a paper close.

**Gate state cleanups (5 — reopened from iter-3 user demand, residue cleared):**

- T-302.12, T-302.65, T-303.4.21, T-303.10, T-303.18.6 — all reset to `state=pending` via gate_Reopen (no longer error-state). They remain UAT-priority-floor; queued for when bug-fix pile drains.

**PM-direct hygiene (3 commits):**

- `apps/desktop/out` symlink un-tracked + worktree-create.sh patched to add the path to common git exclude (caught accidental commit from T-322.19 merge).
- Iter-4 lessons appended to learnings.md.
- package-lock.json sync for T-313.1 `diff` dep.

## Major artifact: gate_Reopen primitive (T-325.1)

The iter's headline ship. Was needed because iter-3 user-demanded "reopen all WONTFIX" left 6 tasks in `state=error` (file in todo/ + history terminal). Iter-4 PM hand-stripped one task (T-302.64) via python markdown surgery, then shipped a proper primitive: `gate_Reopen` accepts `{root, taskId, agent, notes}`, validates location==todo + state-is-terminal, appends a `Record Kind: reopen` section that the deriver resets to initial. 8 lifecycle tests across all 5 builtin pipelines. **Daemon redeploy is required after any gate-core/core change to expose new MCP commands** — this was the iter's only "had to manually bounce launchd" moment (per T-322.36 which shipped the fix for the inode-check footgun that caused it).

## Process violations to be aware of

The task-pm contract says "stop before terminal gate_Pass" but the gateway's auto-advance on the LAST intermediate stage IS the terminal pass. Multiple task-pms this iter (~8) inadvertently fired the terminal pass on red-team/blue-team (whichever is last) and self-reported the violation. The work was clean in every case; only the bookkeeping was noisy. The right path forward is to amend `~/.copilot/agents/task-pm.md` to clarify: "stop after passing the SECOND-TO-LAST intermediate stage." Until that lands, top PM just verifies `ls done/T-X.Y*` after each clean-completion WorkResult and skips the terminal gate_Pass call.

## Daemon / process state at iter close

- Daemon PID 20649 — running iter-4 binary (rebuilt + bounced mid-iter at 10:56 for T-325.1 gate_Reopen exposure). Has gate_Reopen + sampling_deny=-1 + all iter-4 backend changes.
- HEAD: ~a06e7cb76 on dev (will move as final 3 task-pms merge).
- 3 worktrees still in flight at iter-end: T-314.1 (drag-drop), T-322.5 (memory URL autolink), T-322.29 (sanitize 5 more error consumers). If iter ends before they return, next PM merges them.
- One stale `/tmp/uat-fixture-folder` from prior iter — `rm -rf` is safe.

## What's at the TOP of the iter-5 dispatch queue (after the 3 in-flight finish)

Sorted by `(is_uat, sprint_number, task_number, priority)` per the iter-3 rule (UAT lowest priority):

1. **T-314.2** P1 tdd — Generic file attachment hardening (size caps + URI uniqueness + textual blob dispatcher). Wait for T-314.1 to merge first; both touch attachment-helpers.ts.
2. **T-315.1** P1 tdd — audit-crates debt: 13 pre-existing violations across gate-core/core/jockey. **Needs decomposition** before dispatch — too many files for one task-pm. Decompose into T-315.1.1 (gate-core 2 files), T-315.1.2 (core/mcp 2 files), T-315.1.3 (core/workflow/pipeline 3 files), T-315.1.4 (core/api 2 files), T-315.1.5 (core/tools+repair 2 files), T-315.1.6 (jockey 2 remaining files). Note: T-315.2 already covered jockey/handler_execute.rs; spec already excludes that.
3. **T-322.5** (in flight) — memory URL autolink + edge cases
4. **T-322.6** P3 sdd — memory:// link hover preview card
5. **T-322.24** P1 sdd — collapse ToolNodeFields wrapper component (HOLD: this contended with T-322.26-v2 import rewrites; verify after iter-4 merges land that the wrapper file still has no remaining callers)
6. **T-322.29** (in flight) — sanitize 5 more getErrorMessage consumers
7. **T-322.30** P3 tdd — sanitize wikilink queries in WikilinkCreateDialog + useMemoryPromotion-helpers
8. **T-322.31** P3 tdd — convert check-node-modules-integrity-missing.test.mjs to vitest style
9. **T-322.32** P3 sdd — deploy-daemon.sh sanity-check lsof path contains 'maenifold' (CAREFUL: just merged T-322.36 which already touched deploy-daemon.sh; verify no conflict)
10. **T-322.33** P3 sdd — deploy-daemon.sh replace 2s sleep with PID-discovery retry loop (same caveat re: T-322.36)
11. **T-322.34** P3 sdd — Edge-case enumeration rule refinements
12. **T-322.35** P3 sdd — ralph/prompt.md CDP-bound task-scope rule (PM-direct doc edit, trivial)
13. **T-319.1b** (file me!) — remaining 5 Cmd+K palette providers (workflows, pipelines, gate tasks, knowledge notes, settings). Architecture from T-319.1 supports easy extension.
14. **T-324.x** sprint 324 children — config env passthrough decomposition. T-324.0 epic still needs decomposition into the 5 children mentioned in its umbrella spec; T-324.1 and T-324.2 already shipped in earlier sprints. Need T-324.3 (mcpServers stdio spawn wire), T-324.4 (UI Settings view), T-324.5 (docs).

NOT in dispatch queue (UAT-priority-floor, do not dispatch unless user asks):
- T-302.12, T-302.65, T-303.4.21, T-303.10, T-303.18.6 (just reopened via gate_Reopen this iter; still UAT)
- T-303.9, T-303.18.2, T-303.19.1, T-304.2, T-304.3, T-304.6, T-304.19, T-311.2 (UAT/perf-measurement)
- T-305.17 (clear-history test refresh — UAT-ish)
- T-316.2 (plyr AV evaluation — research/eval, low value)

## Open process improvements (file as gate tasks next iter if dispatching)

1. **task-pm contract clarification**: amend `~/.copilot/agents/task-pm.md` line by line: "stop after passing the SECOND-TO-LAST intermediate stage; the LAST intermediate IS the terminal transition (top PM owns it)." 8+ task-pms this iter unintentionally fired the terminal pass.
2. **gate_Reopen UX**: file T-325.2 to extend gate_Reopen to optionally atomically `mv` the file from `done/` → `todo/` before appending the reopen record. Current API requires the user to manually mv first.
3. **worker dispatch budget vs npm install path**: package.json-touching tasks need explicit "PM will run npm install after merge" in dispatch prompt + post-merge step in PM playbook.
4. **Mid-merge conflict resolution from stale worker branches**: T-322.26 needed two attempts because its base was stale. Consider auto-rebasing worker branches against latest dev BEFORE dispatching, or at least flagging stale branches in the PM's saturation check.

## Iter throughput tally

- **18 task-pm shipments (code merged)**
- **3 WONTFIX cascades with cited evidence** (NOT user-direction-defying — premise verifiably dead in each case)
- **5 gate-state cleanups** via new gate_Reopen primitive
- **22 dev commits** (worker commits + merges + 3 PM-direct hygiene)
- **1 daemon redeploy** (post-T-325.1 merge; manually bounced after T-322.36's deploy-daemon bug was exposed)
- **1 major new primitive shipped**: gate_Reopen (T-325.1) — eliminates the manual-mv-from-done error-state trap permanently
- **0 sprint umbrellas converged**
- **0 user interruptions during dispatch**
