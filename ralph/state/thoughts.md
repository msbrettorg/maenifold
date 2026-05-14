# RALPH Thoughts

## Iter-N notes for future iters

- **Sprint 303 status:** T-303.4.5 shipped iter-N+1 (commits da7d74ea6 + 3719926c2 + 41eada65b → merged 925acb8e7). Remaining non-UAT in sprint 303:
  - T-303.4.5.2 (P3) — LookupResult rename + SamplingPromptReason modal context (filed iter-N+1 from red-team round 2 NOTEs).
  - T-303.4.5.3 (P3) — worktree process gap (todo/ gitignored — SWE can't read spec from worktree path).
  - T-303.15.2 (P3, lightweight) — npm install + tools/list smoke test in apps/desktop/test-fixtures (off-limits for personal RALPH SWE — needs broader-scope dispatch).
  - T-303.18.3 (P3, tdd) — frontend per-source MCP prompt enumeration (apps/desktop, off-limits for personal RALPH SWE).
- Sprint 303 RALPH-converged once T-303.4.5.2/.3, T-303.15.2, T-303.18.3 land OR are formally re-scoped to a different RALPH instance.
- **Iter-N+1 process pattern WORKED well:** dispatch inner-loop SWE → PM read-back → fresh code-review red-team (opus-4.6) → on FAIL, `write_agent` follow-up to same SWE for narrow remediation → fresh red-team round 2 → blue-team (inner-loop, gpt-5.5) for tests → PM merge. Single iteration shipped a complex cross-stack slc task with one remediation cycle.
- **Prerequisite handoffs**: T-303.4.5 hard-blocked on T-303.4.16 in iter-N. Iter-N+1 unblocked because T-303.4.16 landed in done/. Verify hard-blocks before each iter starts (read previous thoughts.md AND check done/ for the prereq task file).
- **Architect duo wave-boundary rule continues earning its keep.** The spec for T-303.4.5 went through gpt-5.5 + opus-4.6 rubber-duck reviews in iter-N before SWE dispatch — caught 9+ items reconciled into spec amendments. Without that, SWE would have shipped an even worse first-draft.
- **Process error to avoid:** do NOT call `gate_Pass` twice in rapid succession. The maenifold CLI returns verbose JSON; if your parser silently fails, it does NOT mean the call failed. Always check `gate_Status` immediately after to confirm the transition. See learning #8.
- **Same-SWE remediation pattern:** `write_agent` to the still-idle SWE for narrow remediation is dramatically faster than dispatching fresh. The agent retains the codebase context. Use this for remediation cycles unless the original SWE's work was fundamentally untrustable.

### Standing directives for future iters

- **Lowest-numbered first remains absolute.** Don't skip ahead even if a higher-numbered task looks more interesting.
- **UAT items stay out of RALPH scope.** When you encounter one, leave it in `pending` and move to the next non-UAT item. If UAT is the ONLY remaining work in the lowest sprint, the sprint is RALPH-converged for that iter; move to the next sprint.
- **SLC pipeline spec stage deliverable lives in the task .md file.** Don't create separate spec docs.
- **When dispatching cross-stack SWE work, use `inner-loop` agent + `gpt-5.5` model.** Add explicit "MUST land all changes atomically in one PR" instruction.
- **NEW (iter-N+1):** Always include the MAIN REPO absolute spec path in SWE dispatch prompts (`/Users/brett/src/ma-collective/maenifold-rs/todo/T-XXX.Y-*.md`). Worktree's `todo/` is gitignored — SWE can't read it from the worktree's relative path. Filed T-303.4.5.3 to fix the underlying gap.
- **NEW (iter-N+1):** PM read-back of SWE diff BEFORE commit catches obvious spec omissions earlier than red-team. Don't replace red-team but use this as a coarse first-pass.
- **NEW (iter-N+1):** Same-SWE remediation via `write_agent` is preferred over fresh SWE dispatch for narrow remediation cycles. The agent's full codebase context is preserved.
- **Always check the iter-15 RCA at the bottom of learnings.md** for ralph.sh `perform_rollback` lesson — committed iter work can be silently wiped if SIGTERM lands during cleanup.
