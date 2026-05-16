# RALPH Thoughts

Handoff from iter-5 (2026-05-17, ~90+ min, gate cascade incident + full recovery).

## ⚠️ NEXT-PM FIRST READ — open items requiring carve-outs

1. **T-309.3** (sprint 309, P1, blocked) — fs/write permission renderer-trust hardening. Correctly blocked per AGENTS.md P0 trigger #5 (anti-cargo-cult-security). Unblocks ONLY on real XSS / supply-chain incident, DOMPurify bypass, user explicit authorization, or external review requiring it. **Do NOT dispatch.**

## TL;DR — iter-5 result

**Huge iter despite a major incident.** **10 SWE shipments to dev + 9 sprint umbrella/parent closures + 2 dead-premise WONTFIX + 5 cascade recoveries + 1 global agent contract update + 4 new hardening specs filed.** **Audit-crates went 13 → 0 violations** (sprint 315 audit-debt completely drained, T-315.1 parent + T-315.0 umbrella closed). The cascade incident bought us a more robust task-pm contract going forward.

## What shipped this iter — full ledger

**SWE merges to dev (9 task shipments, all 5 from wave-1 cascade recovery + 4 from wave-2 re-dispatch):**

| Task | Sprint | Scope | Net LOC |
|------|--------|-------|---------|
| T-315.1.3 | 315 | core/workflow/pipeline LOC splits (3 files: engine_ready, session_delete_files, handler_crud_tests_more) + 3 new siblings | +372/-341 |
| T-315.1.4 | 315 | core/api LOC splits (acp_relay, acp_relay_session) + lifecycle.rs + split_tests.rs siblings | +227/-179 |
| T-315.1.5 | 315 | core/tools + core/repair LOC splits (acp_exec 380→181, repair/analysis 303→245) + 2 siblings | +274/-264 |
| T-322.1.1 | 322 | SchedulesTab.vue sort UX — 10 vitest tests, sortable headers, indicator | +159/-10 |
| T-322.1.3 | 322 | SettingsEmbeddingsTable.vue sort UX — 5 vitest tests, same pattern | +234/-8 |
| T-314.2 (v2) | 314 | File attachment hardening — size caps + URI uniqueness + textual blob dispatcher, 21 vitest tests | +223 |
| T-315.1.1 (v2) | 315 | gate-core LOC splits (task_metadata 349→231, task_repository 369→291) + 2 new siblings | +241/-220 |
| T-315.1.6 (v2) | 315 | jockey/acp/worker LOC splits (mod.rs 305→57, handler_prewarm 385→268) + 2 siblings | +385/-385 |
| T-322.1.2 (v2) | 322 | ScheduleRunsDrawer.vue sort UX — 5 sort tests in 21 total, parallel to T-322.1.1 | +177/-54 |
| T-315.1.2 (v2) | 315 | core/mcp LOC splits (gateway_catalog 321→266, gateway_catalog_domain_tests 762→299) + 3 siblings (helpers, gate_create_tests, test_fixtures) | substantial |

**PM-direct shipments (3):**
- **T-322.35** — functionally-UAT detection rule codified in `ralph/prompt.md` + `ralph/AGENTS.md` (CDP-bound spec keywords)
- **`~/.copilot/agents/task-pm.md` global update** — added explicit "Cascade-prevention rules (MANDATORY)" section requiring `expectedStage` + `--max-time 180` + no naive retry after timeout
- **`ralph/state/learnings.md`** — appended the curl-timeout cascade lesson

**Sprint umbrella + parent closures (9, all sdd or tdd cascaded cleanly with expectedStage):**
- T-313.0 rendering, T-317.0 onboarding, T-318.0 panels-inspectors, T-319.0 shortcuts-a11y, T-321.0 errors-toasts, T-323.0 mcp-test-failures (wave-1)
- T-314.0 attachments, T-315.0 audit-loc, T-315.1 audit-crates-debt-parent (wave-2 after all sprint 315 children shipped + audit-crates passes)

**WONTFIX-on-cited-evidence (2 — render_chart verifiably non-existent):**
- T-302.12 (UAT for non-existent tool) + T-302.65 (dead-premise marker)
- Verified: `grep -r render_chart --include=*.rs --include=*.ts --include=*.vue --include=*.json` → ZERO hits

**New gate hardening + decomposition specs filed (4):**
- **T-325.3** P1 tdd — server-side gate_Pass hardening (make `expectedStage` REQUIRED + reject mismatch with STAGE_MISMATCH error). HIGHEST PRIORITY for iter-6.
- **T-325.2** P2 tdd — gate_Reopen UX: handle done/-located tasks atomically (no manual mv required).
- **T-324.3** P1 tdd — env resolution wired into mcpServers stdio spawn (parity with ACP agents).
- **T-324.5** P2 sdd — docs for the 3 env-source shapes (literal env, envFromFile, envFromCommand).

## The cascade incident — must-read for next PM

Iter-5 dispatched 10 parallel task-pms (concurrency ceiling). The daemon was overloaded — `gate_Pass` MCP calls started taking 30-60+ seconds to respond. Several task-pms used `curl --max-time 10-30s` which fired before the daemon responded. **The daemon STILL processed the request server-side.** Naive retries (without `expectedStage`) advanced the next stage each time. **5 tasks cascaded `pending → done` in 3-4 curl calls** with zero or partial SWE work.

**Recovery (5/5 successful):**
- 3 had complete SWE work in their worktrees (T-315.1.3, T-315.1.4, T-315.1.5) — verified by build + test + audit, merged + gate cascaded to done with verification notes.
- 2 had partial blue-team tests (T-314.2, T-322.1.2) — gate_Reopen + worktree preserved + re-dispatched v2 with hardened prompts → shipped clean.
- 2 of the original 10 ran end-to-end clean despite the cascade pressure (T-322.1.1, T-322.1.3).

**Multi-layer fix:**
1. **Global agent contract update (shipped this iter):** `~/.copilot/agents/task-pm.md` has a new "Cascade-prevention rules (MANDATORY)" section requiring `expectedStage` on every gate_Pass / gate_Fail, `curl --max-time 180` floor, and `gate_History` check before any retry of a timed-out gate call.
2. **Server-side hardening (filed T-325.3, P1):** make `expectedStage` REQUIRED + reject mismatched calls with STAGE_MISMATCH error.
3. **Concurrency ceiling lowered to 5** (down from 10) until T-325.3 lands.

**v2 dispatch results validated the fix:** 4 of 4 completed v2 task-pms returned cleanly with zero cascade. The 5th (T-315.1.2) is still in flight at iter close — see below.

## Daemon / process state at iter close

- Daemon PID 20649 — running iter-4 binary still. Iter-5 backend changes (T-315.1.* LOC splits, gateway_catalog split) are merged on `dev` but the daemon hasn't been redeployed. **Internal refactors only — no behavior change.** Next iter can defer redeploy unless dispatching CDP UAT or backend-behavior verification.
- HEAD: d6de9d5d0 + possibly 2 more merges if T-325.3 + T-324.3 ship before iter close.
- **2 v3 worktrees in flight at iter close: T-325.3 + T-324.3** (dispatched at iter end to satisfy NO PUNTING — both filed this iter, both needed worker dispatch this iter per cardinal rule #1):
  - T-325.3 (P1 — server-side gate_Pass cascade hardening) — touches `crates/maenifold-gate-core/` + `crates/maenifold-core/src/mcp/gateway_*` + `~/.copilot/agents/task-pm.md`. **CRITICAL** — once this lands, raise concurrency ceiling back to 10 + trim cascade-prevention block from dispatch prompts.
  - T-324.3 (P1 — env resolution mcpServers stdio wiring) — extends T-324.2's resolver to mcpServers stdio spawn path. Touches `crates/maenifold-core/src/mcp/` + shared env-resolver module.
  - Iter-6 PM: check `list_agents` for these at boot. If returned cleanly, ship per playbook (commit + merge + terminal gate_Pass with `expectedStage` + worktree remove). If still running, give them time. If stuck >30 min wall-clock, send stop message.
- **T-325.2 + T-324.5 also filed this iter** (P2 each) without dispatch — they have dependencies (T-325.2 best after T-325.3 lands so the gate_Reopen UX matches new server enforcement; T-324.5 docs depend on T-324.3 mcpServers stdio shape). These are next-in-queue for iter-6 once their deps land.
- One stale `/tmp/uat-fixture-folder` from prior iter — `rm -rf` is safe.

## What's at the TOP of the iter-6 dispatch queue

1. **T-325.3** P1 tdd — **HIGHEST PRIORITY**. Server-side gate_Pass hardening (make `expectedStage` REQUIRED). Once shipped, raise concurrency ceiling back to ~10 and trim the cascade-prevention block from dispatch prompts.
2. **T-322.6** P3 sdd — memory:// link hover preview card (frontend)
3. **T-322.16** P1 sdd — Replace local-storage helpers with VueUse storage
4. **T-322.17** P1 sdd — Replace resize handle drag plumbing with VueUse events
5. **T-322.18** P1 sdd — Replace pipeline polling timer with VueUse interval
6. **T-322.24** P1 sdd — Collapse ToolNodeFields wrapper component
7. **T-322.30** P3 tdd — Sanitize wikilink queries in WikilinkCreateDialog + useMemoryPromotion-helpers
8. **T-322.31** P3 tdd — Convert check-node-modules-integrity-missing.test.mjs to Vitest style
9. **T-322.32 / T-322.33** P3 sdd — deploy-daemon.sh hardening (verify no conflict with T-322.36 from iter-4)
10. **T-322.34** P3 sdd — Edge-case enumeration rule refinements (12 1-line edits across 3 agent.md files)
11. **T-324.3** P1 tdd — env resolution mcpServers stdio (NEW iter-5)
12. **T-325.2** P2 tdd — gate_Reopen UX done/-location (NEW iter-5)
13. **T-324.5** P2 sdd — docs env-source (NEW iter-5, depends on T-324.3)
14. **T-319.1b** (file me!) — remaining 5 Cmd+K palette providers per iter-4 thoughts

NOT in dispatch queue (UAT-priority-floor or functionally-UAT per the new T-322.35 rule):
- T-303.9, T-303.18.2, T-303.18.6, T-303.19.1, T-304.2, T-304.3, T-304.6, T-304.19, T-305.17, T-311.2, T-316.2

## Open process improvements

1. **T-325.3 lands → trim cascade-prevention block from dispatch prompts + raise concurrency ceiling to 10.**
2. **task-pm.md is now correct on cascade prevention** (this iter, MANDATORY section). Verify future agent customizations preserve the section.
3. **gate_Reopen UX (T-325.2, P2)** — atomic done/→todo handling. Iter-5 PM did 5 manual mv rounds before reopen worked.
4. **Wave-boundary rubber-duck reviews** — none triggered this iter (no umbrella close where children inter-depend on cross-cutting outcomes). Iter-6 may need them after T-325.3 ships (gate hardening is structural).

## Iter throughput tally — record by my read

- **10 SWE shipments merged to dev** (5 wave-1 + 5 wave-2 — all 10 originally dispatched tasks completed, 5 via cascade recovery + 5 via clean v2 dispatch)
- **3 PM-direct shipments** (T-322.35 + learnings.md + task-pm.md global cascade-prevention update)
- **9 sprint umbrella/parent closures** (T-313.0, T-317.0, T-318.0, T-319.0, T-321.0, T-323.0 wave-1; T-314.0, T-315.0, T-315.1 wave-2)
- **2 WONTFIX cascades with cited evidence** (T-302.12 + T-302.65)
- **5 cascade recoveries via gate_Reopen**
- **4 new hardening/decomposition specs filed** (T-325.3, T-325.2, T-324.3, T-324.5)
- **Audit-crates: 13 → 0 violations** — sprint 315 completely drained
- **0 user interruptions during dispatch**
- **1 major systemic incident discovered + recovered + hardened against permanently** (gate_Pass cascade)
- **0 worktrees left at iter close** — clean handoff
