# RALPH Thoughts

## Iter-N notes for future iters

- **Sprint 303 is currently PM's lowest-numbered actionable sprint.** 4 non-UAT items remain; T-303.4.5 + T-303.4.16 both at `swe` state with full implementation specs authored at the bottom of their `todo/T-*.md` files.
- **T-303.4.16 is unblocked** for SWE dispatch. Cross-stack work — needs a broader-scope SWE agent (mcp/ + apps/desktop/ outside personal RALPH SWE scope per `ralph/AGENTS.md`). Recommended dispatch: `inner-loop` agent with `gpt-5.5` model. Spec is at `todo/T-303.4.16-sampling-acp-inspector-wiring.md` lines ~121–end. Verification commands split by gate role per AGENTS.md §Test Policy.
- **T-303.4.5 is HARD-BLOCKED on T-303.4.16.** Spec was passed but the SWE work cannot start until T-303.4.16 completes (T-303.4.5 spec explicitly extends T-303.4.16's `execute_runtime_sampling` signature). Next iteration's PM should NOT dispatch T-303.4.5 SWE until T-303.4.16 is in `done/`.
- **Filed follow-up CDP UAT tasks:** T-303.4.16.1 + T-303.4.5.1 (both pending, blocked on parent completion).
- **Process error to avoid:** do NOT call `gate_Pass` twice in rapid succession. The maenifold CLI returns verbose JSON; if your parser silently fails, it does NOT mean the call failed. Always check `gate_Status` immediately after to confirm the transition. See learning #8.
- **Architect duo wave-boundary rule is earning its keep.** Both spec rounds found multiple BLOCKING issues that would have shipped broken (fabricated error codes, unsound correlator keys, race conditions in session_id_tx fallback). Always invoke duo before passing spec → swe for non-trivial work.

### Standing directives for future iters

- **Lowest-numbered first remains absolute.** Don't skip ahead even if a higher-numbered task looks more interesting.
- **UAT items stay out of RALPH scope.** When you encounter one, leave it in `pending` and move to the next non-UAT item. If UAT is the ONLY remaining work in the lowest sprint, the sprint is RALPH-converged for that iter; move to the next sprint.
- **SLC pipeline spec stage deliverable lives in the task .md file.** Don't create separate spec docs.
- **When dispatching cross-stack SWE work, use `inner-loop` agent + `gpt-5.5` model.** Add explicit "MUST land all changes atomically in one PR" instruction.
- **Always check the iter-15 RCA at the bottom of learnings.md** for ralph.sh `perform_rollback` lesson — committed iter work can be silently wiped if SIGTERM lands during cleanup.

