# RALPH Thoughts

Handoff from iter-13 (2026-05-16, ~3 min). **Seventh consecutive steady-state iter.** Bug-fix pile drained at iter-6; iter-7/8/9/10/11/12/13 all idle. Backlog identical to iter-12. **Iter-7 partial reset still unresolved** — same item as iter-8 through iter-12; surfaced again here. **Wrapper-script auto-short-circuit suggestion still pending user awareness — now 7 iters overdue.**

## ⚠️ NEXT-PM FIRST READ — open items requiring user direction

**1. iter-7 partial reset (CODE/GATE inconsistency — user-input required, UNCHANGED from iter-8 through iter-13).** Reflog shows iter-7 landed [[T-326.1]] (server-side terminal-transition RACI guardrail, 5 files, +146/-2; commits `51ee63987` + merge `e9526f193`) and then wrote a handoff (`6f8dd613f`). Dev was then `git reset` back to `a64e00a07` (iter-6 HEAD). The `done/T-326.1*.md` markdown survives (done/ is gitignored). Result: **gate state says T-326.1 shipped; dev's code does not contain the guardrail.** Iter-8 through iter-13 all left this alone — explicit `git reset` operations are respected absent user direction. Possible user actions:
  - Restore: `git cherry-pick e9526f193` (recover the merge commit; rebuild daemon; redeploy)
  - Reverse gate: `GatewayExecute maenifold.gate gate_Reopen` with from=done (per iter-6 [[T-325.2]]) to move T-326.1 back to `todo/`; re-author or re-dispatch
  - Accept as-is (intentional reset; live without server-side guardrail; gate cache stays inconsistent locally)
  Iter-14 PM: **ask the user** before touching either if not in autopilot. In autopilot, leave alone — do not reverse an explicit user git operation.

**2. [[T-309.3]]** (sprint 309, P1, blocked) — `fs/write permission renderer-trust hardening`. Correctly blocked per AGENTS.md P0 trigger #5 (anti-cargo-cult-security). Unblocks ONLY on real XSS / supply-chain incident, DOMPurify bypass, user explicit authorization, or external review requiring it. **Do NOT dispatch.** Path-constraint defense (`requireNonSymlinkWritePath` + `requireWriteWithinMemory`) is enforced regardless.

**3. UAT authorization (user-input required, UNCHANGED).** The remaining 10 non-T-309.3 tasks are all CDP-bound. Per RALPH protocol UAT-floor + functionally-UAT detection, none get a dispatch slot without user authorization for a UAT pass. Iter-7 verified all 8 UAT-task premises ALIVE (no WONTFIX candidates). User decision needed:
  - Authorize UAT pass: each runs via `scripts/uat-launch.sh <task-id>` in its own worktree with per-task port + TMPDIR + user-data-dir
  - Defer indefinitely: steady-state termination continues each iter

**4. Wrapper-script short-circuit (UNCHANGED from iter-10/11/12/13 — now 7 iters overdue).** Seven consecutive steady-state iters wasted ~3-4 minutes of Copilot CLI inference each (~25-28min cumulative). Concrete `ralph.sh` heuristic: if the last N≥4 commits on `dev` all match `^thoughts: iter-\d+ handoff.*steady-state` AND no new files exist in `todo/` since the oldest of those commits, then `ralph.sh` should print "steady state for N+ iters — user input required" and exit 0 without spawning Copilot CLI. **Not filing a gate task** — the wrapper-script change is out of RALPH autonomous scope (touches the ralph harness itself, which the user owns). If the user wants this implemented, they can authorize a gate task or PM-direct edit.

## TL;DR — iter-13 result

- 0 task closures, 0 SWE dispatches, 0 worktrees, 0 merges, 0 git code commits
- 1 memory note: `memory://mae-auto/iterations/iter13-close-7th-consecutive-steadystate-iter`
- 1 thinking session follow-up (`session-1778966901267-59100` thought 1 + conclusion)
- Backlog state identical to iter-12: 12 pending. Re-verified via gate_Sync + gate_Status.
- Iter-close NO PUNTING audit: PASS vacuously (no filed-but-not-dispatched tasks this iter; only blocked is constitution-hold).
- Wall-clock: ~3 minutes.

## Remaining backlog (12 items, ALL out of autonomous RALPH scope)

**Wait — count check:** iter-12 thoughts said "11 pending"; iter-13 gate_Sync returned `taskCount: 1310` and gate_Status streamed 12 entries from `todo/`. Listing `todo/` directly: 12 files (T-303.18.2, T-303.19.1, T-303.9, T-304.0, T-304.19, T-304.2, T-304.3, T-304.6, T-309.3, T-311.0, T-311.2 = 11 plus `_superseded` directory entry = 11 actual). The iter-12 count of 11 was correct; the gate_Sync `taskCount: 1310` is the historical lifetime task count, not pending. **11 pending — same as iter-12.**

**Constitution-blocked (1):**
- [[T-309.3]] — fs/write permission renderer-trust hardening (P1)

**Sprint umbrellas (2 — cannot close until UAT children land):**
- [[T-304.0]] — sprint 304 umbrella; remaining children T-304.2, T-304.3, T-304.6, T-304.19 (all CDP-bound)
- [[T-311.0]] — sprint 311 umbrella; remaining child T-311.2 (CDP-bound)

**CDP-bound UAT (8 — all premises verified ALIVE iter-7; functionally-UAT skip per protocol):**

| Task | Pri | State | CDP trigger |
|------|-----|-------|-------------|
| [[T-303.9]] | P2 | execute | "Drive each tool via the desktop chat surface" |
| [[T-303.18.2]] | P2 | swe | "Launch packaged Electron app per AGENTS.md UAT" |
| [[T-303.19.1]] | P2 | swe | "Launch packaged Electron app per AGENTS.md UAT" |
| [[T-304.2]] | P1 | swe | "CDP UAT only" |
| [[T-304.3]] | P1 | pending | "agent-browser against the packaged Electron app" |
| [[T-304.6]] | P2 | pending | "Drive via CDP, screenshot each" |
| [[T-304.19]] | P1 | pending | "agent-browser skills get electron / agent-browser connect" |
| [[T-311.2]] | P2 | pending | "CDP-screenshot the rendered output" |

None qualify for WONTFIX (no dead premises). All are functionally-UAT per RALPH protocol's CDP-keyword detection rule.

## Recommendation for iter-14

The board has reached **steady state for the seventh consecutive iter**. Iter-14 PM should:

1. **First action: ask the user** about the iter-7 reset (NEXT-PM FIRST READ item #1) — if interactive. In autopilot, leave alone (do not reverse user's explicit git operation).
2. **Second action: ask the user** whether to authorize a UAT pass for the 8 CDP-bound tasks — if interactive. In autopilot, defer (UAT-floor rule: user must explicitly ask).
3. **Third action: surface the wrapper-script short-circuit suggestion** (NEXT-PM FIRST READ item #4) — if interactive. The user can decide whether to authorize implementing the heuristic. Now 7 iters overdue.
4. **If user defers all three** — there is genuinely no autonomous work. Iter-14 PM closes with a 1-line memory note and ends. This will be the legitimate steady-state termination for the 8th consecutive iter.

## Steady-state convergence (process observation)

**Seven consecutive steady-state iters** (iter-7 idle, iter-8 single WONTFIX cascade, iter-9 through iter-13 all idle) confirms the RALPH loop is operating correctly but the board has converged on questions only the user can answer. The wrapper-script short-circuit suggestion (NEXT-PM FIRST READ #4 above) is overdue by 7 iters — every additional steady-state iter wastes ~3-4 min of inference for the same handoff.

## Process state at iter close

- HEAD before this iter: `cb5371e81` (iter-12 thoughts.md commit). Iter-13 adds only this thoughts.md rewrite.
- Zero worktrees, zero in-flight workers.
- Daemon: confirmed healthy (gate_Sync responded `{taskCount:1310, synced:true}` in this iter). No daemon redeploy this iter.
- Gate cache: 11 pending (unchanged from iter-7 through iter-12).
- Iter-13 single git change: this thoughts.md rewrite (next commit).

## Skill / process improvements (none this iter)

Iter-13 was a steady-state iter: no new prompts authored, no new spec amendments, no agent profile changes. The protocol is correctly identifying functionally-UAT tasks and applying the skip rationale. No new lessons added to `learnings.md` (process state, not durable engineering insight).

## Iter-13 throughput tally

- 0 tasks closed
- 11 pending unchanged
- 0 SWE shipments
- 0 worker dispatches
- 0 worktrees created / destroyed
- 0 git code commits (gate cache change is gitignored)
- 1 memory note: `memory://mae-auto/iterations/iter13-close-7th-consecutive-steadystate-iter`
- 1 thinking session follow-up (`session-1778966901267-59100` thought 1 + conclusion)
- Wall-clock: ~3 minutes (read + verify + memo + handoff + commit)
