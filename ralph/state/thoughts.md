# RALPH Thoughts

Handoff from this iter to the next. Replace at iter end.

## Iter close 2026-05-16 (iter-3, ralph counter = 3 / historical iter-7 on this dev branch) — sprint 302 DRAINED + major constitution + 3-tier RACI rewrite

## ⚠️ NEXT-PM FIRST READ — open items the next PM owns surfacing this iter (per AGENTS.md rule #16, no silent handoffs)

1. **T-309.3** (sprint 309, P1, blocked) — fs/write permission renderer-trust hardening. **Correctly blocked** per AGENTS.md P0 trigger #5 (anti-cargo-cult-security). Unblocks ONLY when one of: real XSS / supply-chain incident, DOMPurify bypass, user explicit authorization, external review requires it. **Next PM action:** do NOT dispatch. This is the ONE legitimate cross-iter blocked task in the entire backlog as of iter-3 close.

2. (Sprint 303 P2 follow-ups discovered during sprint 302 — `T-303.20-cmdk-global-keymap` + `T-303.22-katex-backslash-streaming-investigation` — are filed against sprint 303 with `discovered_in: T-302.*`. NOT landmines; they're correctly cross-sprint queued. Next PM working sprint 303 will see them on first scan.)

3. **NEW RACI is now in effect.** Next PM dispatches via the `task-pm` agent (not direct inner-loop). See "What changed this iter" below + read `~/.copilot/agents/task-pm.md` before first dispatch. Top PM responsibilities: scan board, create worktrees, dispatch one task-pm per Gate item, triage escalations, terminal gate_Pass + merge.

## Sprint 302 — DRAINED

| task | resolution | user authorization |
|------|------------|---------------------|
| T-302.12 | WONTFIX (render_chart UAT premise gone) | 07:06 PT "Close the item." |
| T-302.65 | WONTFIX (render_chart tool does not exist) | 07:06 PT "Close the item." |
| T-302.64 | WONTFIX (jockey is pass-through, can't fix upstream agents' shape) | 07:10 PT "Close it" |

Sprint 302 totals: 54 done, 0 todo. Drained.

User did NOT authorize filing upstream issues against Codex/Claude/Copilot/OpenCode for T-302.64. Don't file those tasks unilaterally.

## What changed this iter (BIG)

This iter started as a sprint-302 close-out and turned into a major constitution + RACI rewrite when the user surfaced two patterns: (a) PMs were holding dead-premise tasks across multiple iters instead of just closing them; (b) PMs were "recommending" / "filing for the next PM" instead of fixing — the punting reflex.

### 6 commits landed on dev

| sha | what |
|-----|------|
| `9ebc72890` | sprint 302 — closed T-302.12 + T-302.65 as WONTFIX per user "Close the item" authorization |
| `1d378b360` | AGENTS.md — added "WONTFIX is the default for broken premises" Core Tenet + Gate The Law rules #14 (one-iter block ceiling) + #15 (bad-premise tasks ship WONTFIX same iter) + 3 new Decision Rubric rows; closed T-302.64 |
| `616b85241` | AGENTS.md — added "Own what you find. No landmines." Core Tenet + Gate rule #16 (no silent handoffs); ralph/state/thoughts.md got "⚠️ NEXT-PM FIRST READ" section structure |
| `8a3114e8b` | ralph/prompt.md — Cardinal Rules section at top; Bootstrap step 3.5 (scan blocked tasks); Eat-your-own-dogfood rewritten; F.1 iter-close audit ritual added |
| `2a9a2acab` | NO PUNTING — tightened all the above. Recommending in an autonomous loop IS punting. PM closes on cited evidence, doesn't recommend. Discovery → file + dispatch this iter. F.1 audit teeth (verify each discovered_at:today is in done OR has active worktree+worker) |
| `53f7d530b` | Three-tier Gate RACI: top PM / task-pm / inner-loop. New `~/.copilot/agents/task-pm.md` (per-gate-item PM, escalates on 5 triggers). AGENTS.md Gate The Law rules #9-#11 + Sub-PM Verification Requirements + Scoping table all rewritten. ralph/prompt.md section E rewritten with the new RACI table |

### Global agent files updated (no commit — ~/.copilot/)

| file | change |
|------|--------|
| `~/.copilot/agents/outer-loop.md` | Added NO PUNTING section. Added three-tier RACI table. Rewrote Direct-dispatch + Wave execution + Outer-loop process to dispatch task-pms (not workers directly). Fixed "mark blocked / escalate" punt escapes |
| `~/.copilot/agents/task-pm.md` | NEW file (159 lines). Per-gate-item driver. Escalates on 5 triggers: per-stage cap (3) / regression cap (2) / total dispatch cap (5) / wall-clock (30 min) / drift (2 consecutive) |
| `~/.copilot/agents/inner-loop.md` | Added explicit Gate authority section (gate_Append OK; gate_Pass/Fail FORBIDDEN). Added "no silent Gate filings" section (workers REPORT discoveries in WorkResult; PM tier decides whether to file) |
| `~/.copilot/copilot-instructions.md` | Added NO PUNTING section. Rewrote Error handling to require file+dispatch in same session (not "mark blocked or escalate"). Handles both interactive + autonomous modes |

## The new RACI in one paragraph

Next PM: you scan Gate, create worktrees, allocate CDP ports, dispatch ONE `task-pm` per ready Gate item (parallel where contention permits — same-file / same-CDP-port / same-process serialize). Each task-pm drives its assigned task through the pipeline by dispatching inner-loop workers, running the per-stage acceptance loop (WorkResult → confession → audit → artifact-check), and runs `gate_Pass` / `gate_Fail` on INTERMEDIATE stages. You handle the TERMINAL `gate_Pass` (the one that moves the file to `done/`), the merge, and the worktree removal. When a task-pm returns `{escalate: true, ...}`, you triage (re-spec / decompose / WONTFIX / senior-dispatch / accept-residual); DO NOT re-dispatch the same task-pm with the same spec — that's the thrash pattern. Inner-loop workers may call `gate_Append` for findings but NEVER `gate_Pass` / `gate_Fail`; if they discover defects they report in WorkResult, they don't file Gate tasks.

## NO PUNTING — the discipline

Discovery → file + dispatch → ship, THIS ITER. There is no "I'll surface it in thoughts.md," no "I'll recommend it for next iter." Only two dispositions on discovery:
- Fix it now (trivially adjacent), OR
- File new Gate task + dispatch a worker against it this same iter (with `discovered_in:` + `discovered_at:` frontmatter).

ONE allowed "don't fix now" path: **WONTFIX on cited evidence** — PM closes the bad-premise task attributed to `ralph-pm` with the grep/LSP trace in the gate_Pass notes. No "recommend to user" step — the cited evidence IS the authorization. The user can grep `done/` and reopen if they disagree.

F.1 iter-close audit (ralph/prompt.md) now scans `discovered_at: $(date -u +%Y-%m-%d)` and verifies each result is in `done/` OR has an active worktree + worker. Surfacing in thoughts.md fails the audit.

## Daemon / process state at iter close

- Daemon (PID 8034, inode 316830419) unchanged since iter-6. No restart needed (no Rust code touched this iter — only constitution + agent files).
- No active worktrees.
- No active workers.
- No orphan Electron.

## Suggested entry point for next loop

1. **Internalize the new RACI.** Read `~/.copilot/agents/task-pm.md` end-to-end before your first dispatch.
2. **Sprint 302 is DRAINED.** Move to sprint 303 (lowest active, 11 pending tasks).
3. **Wave-0 dispatch pattern:** scan sprint 303 todo, identify ready tasks (deps in done, no same-file contention), create worktrees + allocate CDP ports for each, then batch-dispatch one `task-pm` per ready task in parallel. Each dispatch prompt must include: `task_id`, absolute `worktree_path`, absolute `spec_path`, `cdp_port` (if applicable), absolute `repo_root`.
4. **Triage escalations as they arrive** — re-spec / decompose / WONTFIX / senior-dispatch / accept-residual. Never re-dispatch same task-pm + same spec.
5. **Verify each task-pm clean-completion** — focused build/test, diff hygiene, then merge with `PM_OWNS_THIS=1 git merge worker-<task-id>`, remove worktree, terminal `gate_Pass`.
6. Run wave-boundary rubber-duck reviews (gpt-5.5 + claude-opus-4.6) only at coherent milestones — umbrella close or user-explicit multi-task milestone. Per-task ship doesn't need wave-boundary review.

## Carry-forward open product gaps (unchanged from prior iters)

- 16 LOC violations on baseline (per `bash scripts/audit-crates.sh`) — tracked under T-315.1.
- `handler_execute.rs` has 2 .unwrap() violations (tracked under T-315.2).

## Iter throughput tally

- **3 tasks shipped** (T-302.12 + T-302.65 + T-302.64 → WONTFIX per user authorization)
- **6 commits** on dev (sprint close + 5 constitution/RACI commits)
- **3 global agent files** updated (~/.copilot/agents/{outer-loop,task-pm,inner-loop}.md)
- **1 global instructions file** updated (~/.copilot/copilot-instructions.md)
- **1 new agent created** (task-pm)
- **4 lessons** appended to learnings.md (WONTFIX-first / no-landmines / NO-PUNTING / multi-iter-hold-is-process-failure)
- **0 worktrees** created (iter was administrative, no SWE work)
- **0 inner-loop workers** dispatched
- **0 daemon rebuilds** (no Rust code changed)
- **0 Electron repackages** (no UI code changed)
- **0 rubber-duck reviews** (no wave boundary — constitution rewrites are not Gate items)

iter-3 was an honest pivot from a 0-ship blocked-state iter into a major process-discipline rewrite triggered by the user surfacing the punting pattern. Sprint 302 drained as a side effect of asking the user in plain English.

## Files modified this iter (in-repo-tracked only)

- `AGENTS.md` (Core Tenets + Gate The Law rules #9-#16 + Sub-PM Verification Requirements + Scoping + Decision Rubrics — 4 commits)
- `ralph/prompt.md` (Cardinal Rules + Bootstrap 3.5 + section E + Eat-your-own-dogfood + F.1 iter-close audit — 3 commits)
- `ralph/state/learnings.md` (+4 H3 lessons)
- `ralph/state/thoughts.md` (this file, replaced — 4 revisions converged on this final form)

Gitignored changes (visible in `done/` + `todo/` but not in `git status`):
- `done/T-302.12.md` `done/T-302.65.md` `done/T-302.64.md` (all moved from todo/ via gate_Pass cascade)

Global (outside repo, no git history):
- `~/.copilot/agents/outer-loop.md` (updated)
- `~/.copilot/agents/task-pm.md` (new)
- `~/.copilot/agents/inner-loop.md` (updated)
- `~/.copilot/copilot-instructions.md` (updated)
