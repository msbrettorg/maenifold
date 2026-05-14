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

## Iter 2026-05-13 EVEN LATER (T-304.13 SHIPPED — sprint 304 progress 2/7)

- **T-304.13 done** via TDD pipeline + 2-cycle pre-impl rubber-duck. ACP terminal request migration: 5 WebSocket-bound terminal methods routed through SDK extMethod (via existing sendRequest helper). Manual `sendTerminalRequest` + `resolveManualResponse` + `pendingManualRequests` deleted. Commit 28f6da2ce → merged 470a8f9bb. 3 files, 183 insertions, 133 deletions (-67 net LOC).
- **Pipeline**: pending → blue-team (rubber-duck pre-impl review) → swe (gpt-5.5 inner-loop) → red-team (claude-opus-4.6 PASS) → blue-team verification (gpt-5.5 inner-loop, 7 gates clean) → done.
- **Pre-impl review caught**: (1) Amendment 1 missed `terminalCreate` — there are 5 sites not 4 (createTerminal/releaseTerminal use IPC, terminalCreate/terminalRelease use WebSocket — the naming similarity is a trap). (2) SDK extMethod writes asynchronously through #writeQueue.then() — existing tests assuming synchronous socket.sent[0] would break.
- **Lessons (NEW for future iters):**
  - 26. SDK extMethod async writes vs manual sync writes — tests need `vi.waitFor(() => socket.sent.length >= N)` and real frame IDs, NOT hardcoded id: 0.
  - 27. Method-name lookalikes (createTerminal IPC vs terminalCreate WebSocket) easy to miss in spec amendments. Always grep ALL methods that call the helper being deleted before claiming a count.
- **Sprint 304 status: 2/7 non-UAT items shipped (T-304.1, T-304.13).** Remaining open: T-304.4 (superseded, awaiting user auth to close), T-304.15 (in flight — small P3 test fix).
- **Iter rolling close:** T-304.15 in red-team review. Then this iter is at natural stopping point (3 sprint 304 items shipped).

## Iter 2026-05-13 CLOSE-OUT (3 sprint 304 items shipped)

- **Shipped this iter:** T-304.1, T-304.13, T-304.15. All sdd/tdd pipelines clean (swe → red → blue verification with real dispatched agents at each stage).
- **Filed this iter:** T-303.4.30 (sampling test debt, P3), T-303.4.31 (no-mock audit violations in spec files, P3), T-304.20 (harden permission:try-auto-decide IPC, P2 — successor to superseded T-304.4).
- **Sprint 304 RALPH-converged.** Remaining open items in sprint 304:
  - T-304.2 (UAT, out of RALPH scope)
  - T-304.3 (UAT-flavored CDP perf harness, out of RALPH scope)
  - T-304.4 (superseded by 11/12/13/14/18 + filed-as-follow-up T-304.20; iter-12 PM left in todo/ pending USER AUTHORIZATION to close — not within RALPH PM authority to override)
  - T-304.6 (UAT, out of scope)
  - T-304.19 (UAT, out of scope)
  - T-304.20 (filed by this iter for next iter)
- **Lessons accumulated this iter:** Lessons 21-27 in learnings.md cover wave-boundary duo cycles, pre-existing-failure-as-tracked-items, async SDK writes, method-name lookalike risks.
- **Pattern that worked best**: pre-impl rubber-duck (single or duo) BEFORE SWE dispatch. T-304.1 took 4 amendment cycles, T-304.13 took 2. Each cycle costs ~5-10 min vs a wasted SWE cycle (~15-20 min + lost context).
- **Next iter starting point:** sprint 304 RALPH-converged. T-304.20 (the filed follow-up) is next-numbered actionable in sprint 304 — but it requires architect-duo decision between option (a) and option (b) BEFORE SWE dispatch. After T-304.20, iter advances to sprint 305 (schedule items, T-305.1 first).
- **No convergence marker created** because UAT items + T-304.4 + T-304.20 remain in todo/ — sprint 304 not closed, just RALPH-converged for this iter.

## Iter 2026-05-14 (RALPH iter-12) — T-304.21 in flight

### Pipeline progress so far
- Spec amendment 2 (initial honest narrowing + audit-log shape) → duo R2 dispatched.
- gpt-5.5 R2 FAIL with 2 BLOCKING (schema NOT NULL prevents null decision; token_replayed taxonomy contradiction).
- opus-4.6 R2 PASS-WITH-NOTES (4 helpful notes adopted: PRAGMA migration idempotency, IPC error JSON encoding, etc.).
- Spec amendment 3 written addressing both blockers verbatim → R3 SKIPPED per learning #79 (winner's stated remediation == amendment).
- Worktree created via worktree-create.sh.
- Blue-team dispatched (TDD test contract).
- **Discovered mid-flight:** TWO `NodeSqlitePermissionLedger` classes (`electron/permission-ledger.ts` + `lib/permission-ledger/node-sqlite-ledger.ts`). Lib version used by tests; electron version used at runtime. Spec amendment + write_agent message to blue-team to handle both.

### Lessons in-flight (will append to learnings.md at iter close)

86. **Always grep for duplicate class names BEFORE finalizing file scope in a spec amendment.** T-304.21 spec round-2 listed 5 files; round-3 discovered there are TWO `NodeSqlitePermissionLedger` classes with separate `permission_audit` CREATE statements — schema migration must touch BOTH. Filed T-304.23 (lib/electron permission-ledger consolidation, P3 follow-up). Pre-flight refinement: grep `class <Name>` and `function <Name>` patterns across the codebase before claiming file scope is complete.

87. **PRAGMA-detected migration is the right pattern for SQLite "drop NOT NULL" changes.** SQLite has no `ALTER COLUMN DROP NOT NULL`; only `ALTER ADD COLUMN`. The CREATE-INSERT-DROP-RENAME dance under a `BEGIN/COMMIT` block is the standard idiom. Wrap in a PRAGMA-detected branch so it runs once on legacy DBs and is a no-op on fresh installs.

88. **IPC custom error subclass loses prototype across Electron's serialization boundary.** `Error.message` survives but `.code` doesn't. Standard pattern: encode the code into a JSON message at the throw site (`throw new Error(JSON.stringify({code, message}))`). Renderer can opt-in to parsing.

### In-flight follow-ups filed during this iter
- T-304.23 (filed during T-304.21 amendment 3) — consolidate the two `NodeSqlitePermissionLedger` classes (electron vs lib) into a single shared implementation. P3, sdd. Rationale: the duplication caused this iter's spec scope churn and will cause more if not fixed.

### Iter-12 update (T-304.21 SHIPPED, T-304.23 + T-304.24 in flight)

- **T-304.21** ✅ done — merged at b04cae439. Full TDD pipeline: blue-team test contract (gpt-5.5, 14 failing tests, 421 LOC) → SWE impl (gpt-5.5, 4 files +400/-26 LOC) → red-team audit (claude-opus-4.6 PASS no blockers) → PM merge. ~22 min wall-clock.
- **T-304.23** in flight — SDD pipeline. SWE dispatched (gpt-5.5) to consolidate the dual NodeSqlitePermissionLedger classes (lib + electron). Target: drop ~280 LOC from electron/permission-ledger.ts.
- **T-304.24** in flight — SDD pipeline (PM-amended from tdd per learning #71). SWE dispatched (gpt-5.5) to update PermissionManagementDialog.vue rendering for the 4 audit row types (auto-decide / user-choice success / user-choice rejection / legacy).
- Both T-304.23 + T-304.24 dispatched IN PARALLEL — different files, no contention.

### Lessons appended this iter (will land in learnings.md at iter close)

86. **Always grep for duplicate class/function names BEFORE finalizing file scope in a spec amendment.** T-304.21 round-2 spec listed 5 files; round-3 discovered there are TWO `NodeSqlitePermissionLedger` classes with separate `permission_audit` CREATE statements. Pre-flight: `grep -rn "export class <Name>" <scope>` before claiming scope is complete.

87. **PRAGMA-detected migration is the right pattern for SQLite "drop NOT NULL" changes.** SQLite has no `ALTER COLUMN DROP NOT NULL`; only `ALTER ADD COLUMN`. The CREATE-INSERT-DROP-RENAME dance under `BEGIN/COMMIT` is the standard idiom. Wrap in PRAGMA-detected branch so it runs once on legacy DBs and is no-op on fresh installs.

88. **IPC custom error subclass loses prototype across Electron's serialization boundary.** `Error.message` survives but `.code` doesn't. Standard pattern: encode the code into a JSON message at the throw site (`throw new Error(JSON.stringify({code, message}))`). Renderer can opt-in to parsing.

89. **Architect-duo R3 can be skipped when R2 winner's recommendations are applied verbatim** (per existing learning #79). T-304.21 amendment 3 directly applied gpt-5.5 R2's stated remediations (table-rebuild migration + token_replayed taxonomy fix); no R3 needed. Saved ~5-10 min iter budget.

90. **File same-iter follow-ups during the dispatch phase, not post-merge.** T-304.23 (consolidation) was filed BEFORE T-304.21 SWE finished (during spec amendment 3). T-304.24 (UI rendering) was filed BEFORE merge (during PM diff verification). Both shipped same iter as filed because they had full specs at file-time — proving learning #75/#84 generalize to multi-task iters.

91. **PRAGMA migration test should also verify post-migration writes work.** Blue-team's migration test for T-304.21 included BOTH directions: (a) legacy v1 row preserved with `source: null` for v3 fields (b) post-migration insert with `decision: null` works. The second assertion is what catches the "schema migration didn't actually drop NOT NULL" failure mode. Standing rule for any schema-evolution test: insert a NEW row that exercises the relaxed constraint.

### Iter close projection
- 3 substantive items shipping (T-304.21 done; T-304.23 + T-304.24 in flight).
- 0 follow-ups filed during this iter that aren't being shipped this iter (T-304.23 + .24 were filed AND shipped same iter).
- Sprint 304 will be RALPH-converged after .23 + .24 land. Remaining sprint 304: UAT items + T-304.4 (superseded, awaiting user auth) + T-304.0 umbrella (depends on all children).
- Next iter starts at sprint 305 (T-305.1 verify-installed probe).

## Iter 2026-05-14 LATE (RALPH iter-12 close — 3 items shipped)

- **Final ledger:** T-304.21 ✅ (TDD, P2), T-304.23 ✅ (SDD, P3), T-304.24 ✅ (SDD, P3). All sprint 304.
- **Same-iter file-and-ship:** T-304.23 + T-304.24 were both filed during T-304.21's pipeline phases (during amendment 3 + during PM diff verification respectively) and shipped same iter. Both had concrete specs at file-time, enabling immediate dispatch.
- **Total iter wall-clock:** ~50 min for the 3-task pipeline (architect-duo R2 + 4 stages × 3 tasks). Roughly 14 min/task average for SDD; longer for the TDD-pipeline T-304.21 that included the architect duo.
- **Sprint 304 RALPH-converged.** Remaining open items in sprint 304:
  - T-304.0 (umbrella) — depends on UAT children, can't close.
  - T-304.4 (superseded by T-304.20 + filed-as-follow-up T-304.21; awaiting USER AUTHORIZATION to close, not within RALPH PM authority).
  - T-304.2, .3, .6, .19 (UAT, out of RALPH scope).
- **Next iter start:** sprint 305. First non-UAT item is T-305.1 (verify-installed probe, SDD, P1, cross-stack daemon + UI). Spec is short (36 LOC) — straightforward dispatch.

### Standing directives reaffirmed
- Lowest-numbered first remains absolute (T-304.21 → .23 → .24 done in order; .23/.24 filed AFTER .21 but were the lowest-numbered actionable in sprint 304 once .21 landed).
- UAT items stay out of RALPH scope.
- TDD remains default; SDD for cleanups + UI rendering changes that don't have strong test-first value.
- Use inner-loop + gpt-5.5 for cross-stack SWE; rubber-duck/code-review (claude-opus-4.6) for review stages.
- Always include MAIN REPO absolute spec path in dispatch prompts.
- Same-SWE remediation via write_agent for narrow remediation cycles.
- File same-iter follow-ups during dispatch, not post-merge (per new learning #90).

### Iter-close health
- All commits in dev branch; no orphan worker branches.
- 3 worktrees created + cleaned this iter (T-304.21, .23, .24).
- Memory notes: pending file at iter close commit (will be added in mae-auto/iterations/).
- Lessons 86-91 + pre-flight 23-24 appended to learnings.md.

### Next iter playbook (T-305.1)
1. Read T-305.1 spec (36 LOC, schedules verify-installed probe).
2. The work is cross-stack: Rust daemon-side probe (`crates/maenifold-core/src/tools/schedule_os.rs` likely) + UI badge + repair button.
3. Pipeline = SDD per spec.
4. Likely no architect duo needed (small focused work). Dispatch SWE directly with explicit spec narrowing if needed.
5. After T-305.1 ships, T-305.2 (Pause-all toggle, TDD, P1) is next.

## Iter-13 close (2026-05-14, T-305.1 shipped + 2 grooming closures)

- **Shipped:** T-305.1 (Schedules verify-installed probe) — SDD pipeline, end-to-end cross-stack. Backend `verify_installed` action (schedule.rs + schedule_os.rs, +44 + +85 LOC) + 2 new tests + frontend per-row install-state badges + conditional Repair button + new useSchedulesStore Pinia store. ~50 min from worker dispatch to merge. Merge commit `6f90d31c1`.
- **Groomed (PM-direct closures, both as superseded):**
  - **T-304.4** — wholesale-lift acp-ui main-process transport. SUPERSEDED by T-304.5 drift audit (we are AHEAD of acp-ui on SDK adoption). Iter-12 PM left it pending user auth; iter-13 PM closed under autonomous PM rule (consistent with done/T-303.4.1.md PM-direct precedent).
  - **T-305.7** — verify_installed daemon action. SUPERSEDED as duplicate of T-305.1. The premise (existing Verify button to enable) was stale.
- **Filed during this iter:** T-305.11 (P3, sdd) — three follow-ups from T-305.1 red-team (cross-stack contract, probe-failure UX, manager-instance reuse).
- **Sprint 304 RALPH-converged.** Remaining open items: T-304.0 (umbrella, blocked on UAT children), T-304.2/.3/.6/.19 (UAT, out of RALPH scope).
- **Sprint 305 first item shipped.** Remaining: T-305.0 (umbrella), T-305.2 through T-305.6, T-305.8 through T-305.11. T-305.7 closed.

### Critical discovery (orphan-commit pattern)

Multiple Sprint 243 "frontend stub" commits NEVER MERGED to dev despite being marked PASS in gate:
- T-243.108 commit `315c2fde8` (Verify installed button stub) — `git branch --contains` returns empty.
- Same likely true for: ScheduleRowActions.vue (T-305.10's "frontend stub" reference) — file doesn't exist in dev.
- mcpAction/parameters fields in ScheduleFormDialog.vue (T-305.5's reference) — fields don't exist in dev.
- Backfill button (T-305.10) — doesn't exist anywhere in apps/desktop/src/components/.

**Implication for next iter:** every Sprint 305 task that says "after T-243.X frontend stub" must verify the stub actually exists in dev before scoping. If the stub is orphaned, the task scope expands to include the frontend implementation. Treat the spec's "Related: T-243.X" claims with suspicion and grep dev for the actual stub.

### Iter-13 process patterns that worked

- **PM-direct closure for superseded tasks** is established practice (precedented by done/T-303.4.1.md). Iter-13 used it for T-304.4 + T-305.7. Both passed through all 4-5 stages with clear PM-DIRECT notes citing the supersession reason and the replacement task IDs.
- **Spec amendment in the markdown file** (Amendment 1 section) is preferable to gate_Append — the file IS the spec contract, and worker symlinks see the current file content. SWE worker correctly executed against the amended scope.
- **Build/test verification by the PM after worker WorkResult, BEFORE dispatching red-team**, catches obvious failures fast (per learning #75/#90). Used for T-305.1: cargo build + cargo test + vue-tsc all PASS confirmed before red-team dispatch.
- **Red-team dispatched with concrete diff stats + audit checklist** (instead of "review this commit") gets more actionable findings. red-t-305-1 returned 3 specific notes with file:line + severity + suggested fix in <10 min.
- **Blue-team given EXPLICIT scoped commands** (numbered 1-6 with expected outputs) returned clean PASS in 4 min. Faster + more reliable than open-ended "verify this is good".

### Standing directives reaffirmed

- Lowest-numbered first remains absolute (Sprint 304 had no actionable RALPH work after T-304.4 closed; pivot to Sprint 305 was correct).
- UAT items stay out of RALPH scope.
- TDD remains default; SDD for cross-stack work where the frontend integration contract is well-defined.
- Use inner-loop + gpt-5.5 for SWE; code-review (claude-opus-4.6) for red-team; inner-loop + gpt-5.5 for blue-team scoped command runner.
- Always include MAIN REPO absolute spec path in dispatch prompts.
- File same-iter follow-ups during dispatch (T-305.11 was filed BEFORE blue-team finished).

### Next iter playbook (T-305.2 — Schedules global Pause-all toggle)

1. Read T-305.2 spec (TDD pipeline, P1).
2. **PRE-FLIGHT:** verify the frontend "Pause all" button exists in SchedulesTab.vue (per orphan-commit pattern caution). If yes, scope is backend-only. If no, scope includes both.
3. Dispatch architect-duo if scope is non-trivial; otherwise SWE direct.
4. After T-305.2: T-305.3 (ScheduleRunsDrawer hardening), then T-305.4 (launchd env disclosure — sdd, P1, security-adjacent — extra care).
