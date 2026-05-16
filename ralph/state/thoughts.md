# RALPH Thoughts

Handoff from iter-8 (2026-05-16, ~10 min). **Bug-fix pile remains dry** (iter-6 + iter-7 drained it; iter-8 closed one stale follow-up). Remaining backlog is 100% CDP-bound UAT (8) + constitution-hold (1) + 2 sprint umbrellas waiting on their UAT children. **One critical user-input-required item:** iter-7's [[T-326.1]] code + handoff commit were RESET out of dev but the gate cache still records T-326.1 as `done/`.

## ⚠️ NEXT-PM FIRST READ — open items requiring user direction

**1. iter-7 partial reset (CODE/GATE inconsistency — user-input required).** Reflog shows iter-7 landed [[T-326.1]] (server-side terminal-transition RACI guardrail, 5 files, +146/-2; commits `51ee63987` + merge `e9526f193`) and then wrote a handoff (`6f8dd613f`). Dev was then `git reset` back to `a64e00a07` (iter-6 HEAD). The done/ markdown files survive (`done/T-326.1-terminal-transition-raci-guardrail.md`, `done/T-303.18.6.md`) because `done/` is gitignored. Iter-7's `done/T-303.18.3.md` work also survives but DID land in dev legitimately (merge `3facb647f` predates the reset). The result: **gate state says T-326.1 shipped; dev's code does not contain the guardrail.** The next task-pm RACI drift incident will not be caught by the server-side check the gate claims is live. Possible user actions:
  - Restore: `git cherry-pick e9526f193` (recover T-326.1 merge commit; rebuild daemon; redeploy)
  - Reverse gate: `mv done/T-326.1*.md todo/T-326.1-terminal-transition-raci-guardrail.md` then `gate_Sync rebuild=true`; re-author or re-dispatch
  - Accept as-is (intentional reset; live without server-side guardrail)
  Iter-9 PM: **ask the user** before touching either.

**2. [[T-309.3]]** (sprint 309, P1, blocked) — `fs/write permission renderer-trust hardening`. Correctly blocked per AGENTS.md P0 trigger #5 (anti-cargo-cult-security). Unblocks ONLY on real XSS / supply-chain incident, DOMPurify bypass, user explicit authorization, or external review requiring it. **Do NOT dispatch.** Path-constraint defense (`requireNonSymlinkWritePath` + `requireWriteWithinMemory`) is enforced regardless.

## TL;DR — iter-8 result

- 1 task closed (WONTFIX-on-cited-evidence cascade): [[T-305.17]] (refresh-count assertion already shipped at `apps/desktop/src/components/__tests__/ScheduleRunsDrawer.test.ts:303,308`; CDP UAT portion is UAT-floor skip)
- 0 SWE dispatches, 0 worktrees, 0 merges
- 1 critical finding surfaced for user: iter-7 partial reset (above)
- Iter-close NO PUNTING audit: PASS (no filed-but-not-dispatched tasks this iter; only blocked task is constitution-hold)

## Remaining backlog (11 items, ALL out of autonomous RALPH scope)

**Constitution-blocked (1):**
- [[T-309.3]] — fs/write permission renderer-trust hardening (P1)

**Sprint umbrellas (2 — cannot close until UAT children land):**
- [[T-304.0]] — sprint 304 umbrella; remaining children T-304.2, T-304.3, T-304.6, T-304.19 (all CDP-bound)
- [[T-311.0]] — sprint 311 umbrella; remaining child T-311.2 (CDP-bound)

**CDP-bound UAT (8 — all premises verified ALIVE; functionally-UAT skip per protocol):**

| Task | Pri | State | CDP trigger | Premise still alive (surface) |
|------|-----|-------|-------------|------------------------------|
| [[T-303.9]] | P2 | execute | "Drive each tool via the desktop chat surface" | 6 MCP fixtures inventoried iter-7 |
| [[T-303.18.2]] | P2 | swe | "Launch packaged Electron app per AGENTS.md UAT" | McpPromptPicker.vue + SchemaFormRenderer.vue |
| [[T-303.19.1]] | P2 | swe | "Launch packaged Electron app per AGENTS.md UAT" | useMcpCompletion.ts + mcpClientOps.ts + serverCapabilities.completions check |
| [[T-304.2]] | P1 | swe | "CDP UAT only" | useChatReconnect.ts + session-reconnect.test.ts (iter-7 confirmed premise drift was UAT fixture gap, not feature gap) |
| [[T-304.3]] | P1 | pending | "agent-browser against the packaged Electron app" | composables for H2/H3/H4/H6/H7 hypotheses live |
| [[T-304.6]] | P2 | pending | "Drive via CDP, screenshot each" | ChatContentBlocks.vue + resource-mime-dispatcher.ts + useMermaid.ts |
| [[T-304.19]] | P1 | pending | "agent-browser skills get electron / agent-browser connect" | HomeComposer.vue + SessionRowActions.vue (T-304.16/.17 ship) |
| [[T-311.2]] | P2 | pending | "CDP-screenshot the rendered output" | /Users/brett/.copilot/skills/visual-explainer dir present |

None qualify for WONTFIX (no dead premises). All are functionally-UAT per RALPH protocol's CDP-keyword detection rule.

## Recommendation for iter-9

The board has reached **steady state**: every remaining task gates on a decision only the user can make. Iter-9 PM should:

1. **First action: ask the user** about the iter-7 reset (NEXT-PM FIRST READ item #1). Plain-English question with numbered options. Without that decision, the gate/code inconsistency rots in place.
2. **Second action: ask the user** whether to authorize a UAT pass for the 8 CDP-bound tasks. Each runs in its own worktree via `scripts/uat-launch.sh <task-id>`. Drains sprint 304/311 umbrellas.
3. **If user defers both** — there is genuinely no autonomous work. Iter-9 PM closes the iter with a 1-line memory note and ends. This is the legitimate steady-state termination per RALPH protocol; not punting.

## Process state at iter close

- HEAD: `a64e00a07` "thoughts: iter-6 handoff — 19 task ships + 3 sprint closures + zero stuck worktrees" (iter-6 commit; iter-7 was reset out of dev)
- Zero worktrees, zero in-flight workers
- Daemon: PID 43641, iter-5 binary, T-325.3 expectedStage enforcement live, T-326.1 terminal-transition guardrail NOT in binary (reset)
- Gate cache: 11 pending after iter-8 cascade (down from 12)
- Iter-8 single git change: this thoughts.md rewrite (next commit)

## Skill / process improvements (none this iter)

Iter-8 was a steady-state iter: no new prompts authored, no new spec amendments, no agent profile changes. The protocol is correctly identifying functionally-UAT tasks and applying the skip rationale. The single WONTFIX cascade applied the standard Cardinal Rule #2 evidence chain.

## Iter-8 throughput tally

- 1 task closed (T-305.17 WONTFIX cascade through tdd pipeline)
- 11 pending → 10 actionable (T-309.3 stays blocked; 8 UAT-floor; 2 umbrellas)
- 0 SWE shipments
- 0 worker dispatches
- 0 worktrees created / destroyed
- 0 git code commits (gate cache change is gitignored)
- 1 memory note: `memory://iter8-close-bugfix-pile-dry-1-wontfix-cascade-iter7-reset-discovered`
- 1 critical discovery surfaced: iter-7 partial reset
- Wall-clock: ~10 minutes (read + verify + cascade + handoff)
