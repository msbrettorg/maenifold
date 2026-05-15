# RALPH Thoughts

Handoff from this iter to the next. Replace at iter end.

## Iter close 2026-05-15 (iter-2) — sprint 301 carry-forward 4/4 shipped

**Working tree clean** after this commit. dev branch up to date at aae3e48c9.

Backlog: 80 → ~80 in sprint 301 totals (4 shipped + 1 follow-up filed). Net iter throughput: 4 tasks closed (T-301.32, T-301.37, T-301.38, T-301.39).

## In flight at handoff

Nothing. All 4 dispatched workers landed + audited + merged + gate-advanced. All 4 worktrees cleaned up.

## What landed this iter (in commit order)

- **Sprint 301** (carry-forward from iter-1):
  - T-301.32 — fix two baseline maenifold-core lib test failures (gateway_catalog whitelist + sampling deny -32600→-1)
  - T-301.37 — T-301.31 follow-ups (3 sub-tasks): jockey adapter.rs LOC 306→267, docs/DEVELOPMENT.md optional envFromFile schema, pipeline-driver Stuck-status mapping for preflight env failures
  - T-301.38 — fork dialog silent-fail wrapped in toast.error
  - T-301.39 — promote-to-memory empty Connections gets visible validation (toast + focus + danger-styled inline error)
- **Build artifacts refreshed** post-merge:
  - target/release/maenifold + target/release/maenifold-pipeline-driver rebuilt at 13:52
  - apps/desktop/out/maenifold-darwin-arm64/ repackaged at 13:53

## Sprint 301 carry-forward (1 task — filed during this iter)

| Task | Pri | Why deferred |
|---|---|---|
| T-301.40 | P2 | NEW. T-301.37 follow-ups: (a) integration test against a REAL pipeline run with envFromFile pointing at /nonexistent/path (current test asserts unit-level mapping only), (b) structured PreflightFailure variant handling instead of literal string match (fidelity loss at jockey→driver boundary), (c) Err-path coverage in promote_preflight_env_failures_to_stuck (currently only runs on Ok(execution_result) branch). Heavy task; needs new pipeline-driver integration test infrastructure under crates/maenifold-pipeline-driver/tests/. |

## Other open product gaps

- **16 LOC violations on baseline** (per `bash scripts/audit-crates.sh`), down from 17 — T-301.37 sub-task 1 fixed adapter.rs. T-315.1 still owns the full audit-crates debt with 16 violators including handler_execute.rs (455 LOC, +2 .unwrap() violations). Each remaining file should be a tracked task per AGENTS.md "every audit failure is a tracked item."
- **Stale daemon** at iter close: PID 96954 still has the iter-1 binary (inode 314208435) in memory. The new release binary is at target/release/maenifold (built 13:52). Next iter MUST restart daemon before:
  - Pipeline-driver UAT (would test old recovery.rs without the preflight promotion)
  - Sampling-deny verification (old daemon returns -32600, new code returns -1)
  - Any work that depends on T-301.32 or T-301.37 backend changes being live
  Use `./rebuild.sh` (rebuild + launchctl kickstart) or kill PID 96954 + relaunch via launchd.
- **Stale Electron app** (already repackaged but the running app may not be the fresh package). Next iter CDP UAT on T-301.38/T-301.39 features should launch via `scripts/uat-launch.sh <task-id>` against the freshly-packaged binary at apps/desktop/out/maenifold-darwin-arm64/maenifold.app.

## Process insights worth carrying forward

- **TDD pipeline stage collapse continued**: PM dispatched single inner-loop SWE per task with explicit "write failing test FIRST" discipline; SWE confession + tool chronology validated test-first; red-team validated meaningful (not tautological). PM advanced blue-team + swe stages with attribution to "swe-T-XXX (gpt-5.5, TDD test-author phase)" + "swe-T-XXX (gpt-5.5)" — process gap recorded for transparency, see learnings.md.
- **Red-team scoping override**: red-T-301.32 returned FAIL on a pre-existing 757-LOC overage. PM overrode per AGENTS.md "pre-existing → informational, not FAIL" rule. Documented in gate notes + learnings.md. Future red-team prompts should include the explicit instruction to NOT FAIL on pre-existing-baseline files unless the SWE delta crossed the limit boundary.
- **Worker-flagged baseline regressions**: All 4 SWE workers reported full-suite failures (acorn missing + Vue composableIntegration flake + Rust concurrent_read_resource flake) as "blocker requires rework." PM verified each was pre-existing baseline. Workers were honest about uncertainty; PM evidence resolved. Pattern to encode: SWE prompts should explicitly require running the failing test ON DEV BEFORE the change to baseline whether it's pre-existing.

## Suggested entry point for next loop

Per "lowest-numbered first" — start with **T-301.40**. It's the only sprint-301 task remaining. Heavy lift (new integration test infrastructure for pipeline-driver), so allocate generously and consider splitting:
  - T-301.40a: integration test scaffolding + the misconfigured-agent test (P1)
  - T-301.40b: structured PreflightFailure variant refactor (P2)
  - T-301.40c: Err-path coverage in promotion logic (P2)

If sprint 301 closes (T-301.40 done), **iter ends** per "When you finish the sprint you're done with the iteration too." Then sprint 302 next — first task is T-302.11 (Agents tab full UAT, gsd pipeline). Sprint 302 is heavy on CDP UAT items (T-302.11, T-302.12, T-302.35); each needs a CDP-port-isolated worktree and serial execution per "one agent per CDP session."

## Critical pre-flight before next iter starts

1. **Restart daemon** (`./rebuild.sh` or kill PID 96954 + launchd kickstart). Verify with `lsof -p $(pgrep -f 'target/release/maenifold' | head -1) | grep "txt.*target/release/maenifold$"` shows the new inode (compare against `ls -la target/release/maenifold`).
2. **Run `bash scripts/audit-crates.sh`** to confirm baseline is 16 (not 17) post-T-301.37 merge. If 17, the merge is broken.
3. **Run `cargo test -p maenifold-core --lib -- reverse_mapping_exhaustively_covers sampling_deny_returns_negative_1`** to confirm both T-301.32 fixes are in dev. Both should PASS.
