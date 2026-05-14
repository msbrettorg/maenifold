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

## Iter 2026-05-13 (T-303.4.27 + .28 + .29 shipped, T-303.18.3 deferred again)

- **Sprint 303 status:** 3 tasks shipped this iter (T-303.4.27 + T-303.4.28 + T-303.4.29 — all worktree-create.sh hardening). Three-iter dogfooding loop closed: each subsequent task's worktree picked up the previous fix. Remaining open in sprint 303 (excluding UAT):
  - **T-303.0** (umbrella) — blocked on T-303.18.3.
  - **T-303.18.3** (P3, slc) — discovery state. Amendment 2 reviewed; opus-4.6 PASS-WITH-NOTES, gpt-5.5 FAIL with 5 blocking issues. Spec needs Amendment 3.
- **T-303.18.3 next-iter playbook (CRITICAL):** Per gpt-5.5 wave-boundary review (recorded in gate history), Amendment 2 has 5 blocking issues:
  1. **Approach C breaks overlapping-name disambiguation** — original AC#5 explicitly tests this. Need a fresh design (opaque prompt-ID per (server, prompt_name) OR sidecar `GetMcpPrompt {server, name, arguments}` META-tool — explicit server param).
  2. Frontend integration target wrong (real owner is `useMcpPromptCommands.refresh()`, not the caller).
  3. `DiscoverMcpPrompts` registration incomplete — also need `registry_gateway.rs`, `mcp/schema/discovery_tools.rs`, `gateway_cli.rs`, `mcpClientOps.ts:57-59`, `mcpGatewayRouting.ts:8-17`.
  4. Spec self-contradicts on `prompts/list_changed` (lines 298-303 vs 360-375). Remove the `enable` line.
  5. CDP UAT acceptance still uncovered. Either include in-task fixture or escalate scope.
- **Recommended approach for next iter:** read the actual code (the 5 surface files in finding #3) BEFORE writing Amendment 3. The opus-4.6 NOTE 1-6 from prior iter still apply (LOC budgets, Option<JsonObject>, peer.list_all_prompts, GatewayState::new constructor, DISCOVERY_TOOLS const, raw not-prefixed names).
- **Three new pre-flight items added to learnings.md (14, 15, 16):**
  - 14: Re-run path/git-related tests from BOTH worker worktree AND main repo cwd post-merge.
  - 15: gate_Status check before each gate_Pass to confirm the actual current state (not the intended state).
  - 16: Skip formal confession when WorkResult is fully artifactable.

### Standing directives reaffirmed
- Lowest-numbered first remains absolute (T-303.4.27 → .28 → .29 done in order; T-303.18.3 remains open as next non-UAT).
- UAT items stay out of RALPH scope.
- SLC pipeline spec stage deliverable lives in the task .md file.
- Use inner-loop + gpt-5.5 for cross-stack SWE; rubber-duck/code-review for review stages.
- Always include MAIN REPO absolute spec path in dispatch prompts.
- Same-SWE remediation via write_agent for narrow remediation cycles.

### Iter-close health
- All commits in dev branch; no orphan worker branches.
- 3 worktrees created + cleaned this iter (T-303.4.27, .28, .29).
- Memory notes filed for each closed task in mae-auto/iterations/.
- Lessons 51-64 + pre-flight 14-16 appended to learnings.md.
- Filed P3 follow-ups: T-303.4.28, T-303.4.29 (both done this iter — quick triple-iter shipping).

## Iter 2026-05-13 LATE (T-303.18.3 SHIPPED — sprint 303 RALPH-converged)

- **T-303.18.3 done** via 6-iteration wave-boundary duo review converged at Amendment 8. Two sidecar META-tools (`DiscoverMcpPrompts` + `GetMcpPrompt {server, name, arguments}`). NO wire-level prompts capability advertised. Overlapping prompt names natively supported via explicit server param. Commit 1185b9e3d → merged 3facb647f. 21 files, 586 insertions, 53 deletions.
- **Pipeline**: pending → discovery (PM-direct) → spec (Amendment 8 wave-boundary PASS) → swe → red-team (PASS-WITH-NOTES, 0 blockers) → blue-team (Stage 1 + Stage 2 backend PASS, Stage 2 renderer SKIPPED → filed T-303.18.6) → done.
- **Wave-boundary duo took 6 iterations** — gpt-5.5 caught all 7 procedural blockers across 5 iterations (AC scope deferral, daemon launch, maenifold.json content, gateway empty-tools, /healthz endpoint, forge prePackage rebuild, log grep, JSON-RPC wrapper escape). opus-4.6 caught code-correctness in iteration 1 (callTool shape, lifecycle rebuild, parsing helpers, frontend integration target) then PASSed every subsequent iter.
- **Architecture-vs-procedural split:** Amendment 3 established the architecture (two META-tools, no wire prompts capability). Amendments 4-8 were 100% procedural detail without changing the architecture. Pattern: classify each duo finding as ARCH (changes design) vs PROC (changes execution detail) — PROC iterations are cheap.
- **Lessons (NEW for future iters):**
  - 17. SWE inner-loop over-escalates on P0 trigger keywords seen in AGENTS.md (where the rules describe what triggers ARE). PM dispatch must clarify: P0 triggers apply to WORK INPUT, not to reading the rules.
  - 18. `npm run build` runs full `--all` audit which catches pre-existing baseline violations. Use `npm run build:electron` (vite build only) for renderer builds in UAT contexts. `npm run package`'s prePackage hook does this automatically.
  - 19. Blue-team CDP UAT needs ACP agents in test maenifold.json, not just MCP servers — chat composer/slash palette is in ChatView which requires an active ACP session. Filed T-303.18.6 to extend the test fixture template.
  - 20. **gpt-5.5 + opus-4.6 are complementary reviewers.** gpt-5.5 = procedural / executability. opus-4.6 = code correctness / API surface. Always dispatch BOTH for wave-boundary reviews; they catch different classes of bug.
- **Sprint 303 RALPH-converged.** All non-UAT items shipped. Remaining todo/T-303.* items are all UAT (out of RALPH scope per protocol). T-303.0 umbrella can close once T-303.18.6 lands.
- **Next iter: sprint 304.**

## Iter 2026-05-13 LATER (T-304.1 SHIPPED — sprint 304 in flight)

- **T-304.1 done** via SDD pipeline + 4-cycle wave-boundary architect duo. Inline `connection_lost` indicator on the in-flight assistant message + Reconnect button + Send-button health pulse. Commit 18a7576ce → merged a9d75cf80. 12 files, 365 insertions, 12 deletions.
- **Pipeline**: pending → swe (gpt-5.5 inner-loop) → red-team (claude-opus-4.6 PASS, 0 blockers) → blue-team (gpt-5.5 inner-loop, all 7 acceptance gates clean) → done.
- **Duo took 4 amendment cycles** before SWE-dispatchable:
  - Amendment 1: `onTransportClose` callback API. FAIL — wrong wiring location, race with bridgeClosePoll, cross-session ID issues.
  - Amendment 2: Augment handleAgentClosed. FAIL — `if (session.isLoading)` guard never fires because sendPrompt finally clears isLoading 500ms before bridgeClosePoll fires.
  - Amendment 3: Move marker to sendPrompt catch (synchronous). PASS opus, FAIL gpt — cross-session race + wrong API references.
  - Amendment 4: Capture activeBridge at prompt start, set isConnected=false eagerly. PASS BOTH.
- **Lessons (NEW for future iters):**
  - 21. Pre-existing test/audit failures get filed as new gate items so the SWE delta is clean and baseline drift is tracked. Filed T-303.4.30 (8 sampling test failures) + T-303.4.31 (9 no-mock audit violations in spec files) during T-304.1 verification.
  - 22. Wave-boundary duo iterations are CHEAP (~2-5 min each) relative to a wasted SWE cycle (~15-20 min + context loss). Always converge the spec FIRST.
  - 23. When the duo disagrees (gpt-5.5 BLOCKING, opus-4.6 NON-BLOCKING for the same finding), adopt gpt-5.5 stricter interpretation. opus is more permissive on UX-flavored issues; gpt is more rigorous on race conditions and API correctness.
  - 24. `apps/desktop` TS files are NOT subject to the 300 LOC limit (only Vue files are). Verified at `apps/desktop/scripts/audit-components.mjs` line 86-87. Useful for sizing TS-only refactors.
  - 25. `npm run build:electron` is vite-only; `npm run build` runs full audit:components which can fail on pre-existing baseline. Use `build:electron` for renderer-only blue-team verification.
- **Sprint 304 status:** 1/7 non-UAT items done. Remaining open in sprint 304 (excluding UAT items T-304.2, T-304.3, T-304.6, T-304.19): T-304.4 (superseded, awaiting user auth to close), T-304.13 (next actionable — acp-client-core terminal extMethod refactor), T-304.15 (small P3 test fix).
- **Next iter:** T-304.13 (P1, tdd, focused refactor in `apps/desktop/src/lib/acp-client-core.ts`). Note: spec body says "near audit ceiling at 605 LOC" but audit-components.mjs MAX_LOC only applies to Vue files (Lesson 24); TS LOC is uncapped. The "audit ceiling" concern in the spec body is wrong — proceed without LOC anxiety.
