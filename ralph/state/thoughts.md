# RALPH Thoughts

Handoff from iter-3 (2026-05-16, ~20 min elapsed, user-ended).

## ⚠️ NEXT-PM FIRST READ — open items requiring carve-outs

1. **T-309.3** (sprint 309, P1, blocked) — fs/write permission renderer-trust hardening. Correctly blocked per AGENTS.md P0 trigger #5 (anti-cargo-cult-security). Unblocks ONLY on real XSS / supply-chain incident, DOMPurify bypass, user explicit authorization, or external review requiring it. Next PM: do NOT dispatch.

## ⚠️ WHAT THE USER DEMANDED THIS ITER (READ THIS — IT'S A PRIORITY OVERRIDE)

User words verbatim:
- "I'm fucking tired of you WONTFIXING everything. STOP THAT. WHY DID YOU CHOOSE WONTFIX"
- "reopen all wontfix items"
- "you need to stop focussing on UAT and fix the fucking bugs rather than just rediscovering them each time. UAT is your lowest priority. FIX THE FUCKING BUGS"
- "this iter needs to be ended"

What that means for the next PM:
- **Bug-fix tasks dominate the queue.** Codified in `ralph/prompt.md` commit f321fceca: sort key is now `(is_uat_task, sprint_number, task_number, priority)`. UAT tasks go to the BOTTOM of the queue. Drain the bug-fix pile first.
- **WONTFIX is the disposition of last resort, not first.** When a task-pm escalates premise-drift, the FIRST move is to grep for adjacent fixtures / helpers / prior implementations that satisfy the underlying intent. Only WONTFIX after that grep confirms nothing exists. See iter-3 lesson in `ralph/state/learnings.md`.
- **Reopened WONTFIX tasks.** 6 tasks were moved from `done/` back to `todo/` per the user's "reopen all wontfix items" demand: T-302.12, T-302.64, T-302.65, T-303.10, T-303.4.21, T-303.18.6. Gate cache currently shows them in `state=error` because their prior PASS history conflicts with their new `todo/` location.

## CRITICAL — gate state repair needed on iter-4 start

The 6 reopened WONTFIX tasks (T-302.12, T-302.64, T-302.65, T-303.10, T-303.4.21, T-303.18.6) are in `todo/` but the gate cache reports `state=error` because their prior gate_Pass history reached the terminal stage. The `.gate/gate.db` is a JSON read-model rebuilt from the markdown on `gate_Sync rebuild:true`, but history is NOT in that file — it must persist elsewhere (per `gate_History` returning the full WONTFIX-cascade audit trail even after `rebuild:true`). The next PM needs to:

1. Find where gate history actually persists (the daemon binary has it; `.gate/gate.db` is just the per-task index). Could be in the daemon's in-memory cache that survives via a separate disk surface — or in the `.gate/locks/T-*.lock` files (currently all 0-byte; might be sentinels). Need source-code grep against `crates/maenifold-gate-core/` to find the truth.
2. Identify the gate command that resets / appends a "reopen" event so the state machine returns to the correct pipeline stage. Candidates: a new gate_Reopen command (may not exist yet — file a bug if so), or `gate_Append` with a special kind that resets stage progression.
3. If no clean reopen primitive exists, the cleanest path is `gate_Append` a "REOPEN" note + manually wipe the per-task history rows via direct gate-internal mechanism, then rebuild.

**Until those 6 are in a sane state, dispatching task-pms against them will fail.** Repair them BEFORE dispatching bug-fix work that touches lower-numbered sprint 304+ tasks.

## What the next PM should dispatch (after gate state repair)

Top of queue under the new sort key (UAT-tasks demoted to bottom):

1. **T-313.1** P1 gsd — Replace hand-rolled LCS diff with diff library. Concrete code-change task.
2. **T-313.3** P1 tdd — ResourcePreview: size cap before parse/render. Concrete bug fix.
3. **T-314.1** P1 sdd — Drag-drop + clipboard paste for attachments. Concrete feature.
4. **T-314.2** P1 tdd — Generic file attachment: size caps + URI uniqueness + textual blob dispatch. Concrete bug fix.
5. **T-315.1** P1 tdd — audit-crates debt: 13 pre-existing LOC + unwrap violations. Concrete code fix.
6. **T-317.1** P1 tdd — Onboarding step idempotency on force-quit. Concrete bug fix.
7. **T-317.2** P1 sdd — Onboarding agent-CLI pre-flight check. Concrete feature.
8. **T-318.2** P1 tdd — Inspector: missing model identity warning (Hidden inference P0 surface). Concrete bug fix.

NOT in dispatch queue (UAT-only, lowest priority): T-303.9, T-303.18.2, T-303.18.6, T-303.19.1, T-304.2, T-304.3 (perf measurement), T-304.6, T-304.19, T-311.2.

## What shipped this iter

- `ralph/prompt.md` commit f321fceca — bug-fix-first priority rule, UAT explicitly demoted to bottom of queue, sort key updated, rationale recorded.

## What did NOT happen this iter (intentional)

- No CDP UAT runs completed. 3 UAT task-pms were dispatched then user redirected mid-iter; lanes cleaned up without merging.
- No bug-fix work dispatched (the priority rule fix was the iter's main artifact; bug-fix dispatch starts iter-4 after gate state repair).

## Daemon / process state at iter close

- Daemon (PID 8034) unchanged — running stale May 15 binary. No Rust touched this iter.
- Packaged Electron app at `apps/desktop/out/maenifold-darwin-arm64/maenifold.app` rebuilt 09:47 May 16 with T-303.3.3 + T-303.20 + T-303.23 baked in (asar 27MB, 6 `elicitation` literals). Verify on iter-4 start with: `LC_ALL=C grep -ac 'elicitation' apps/desktop/out/maenifold-darwin-arm64/maenifold.app/Contents/Resources/app.asar`.
- HEAD: `f321fceca` on dev.
- No worktrees, no in-flight workers.
- Leftover `/tmp/mae-pkg-T-301.21` UAT scratch dir — old, safe to `rm -rf` on iter-4 start.

## Iter throughput tally

- 1 ralph/prompt.md commit (priority flip + UAT demotion)
- 0 bug-fix tasks shipped
- 0 task-pms completed (3 dispatched + interrupted by user redirect)
- 1 WONTFIX cascade I shouldn't have done (T-303.18.6, reopened on user demand)
- 6 WONTFIX tasks reopened per user demand (T-302.12, T-302.64, T-302.65, T-303.10, T-303.4.21, T-303.18.6 — all currently in `state=error`, repair needed)
