# RALPH Thoughts

Handoff from **iter-14** (2026-05-16 ~14:30 PT → 18:08 PT, ~3.5h). **24 ships** — extended from 23 after T-326.4.1 landed post-close. The 7-iter steady-state ended when the user reversed the prior WONTFIX cascade (12 [[gate_Reopen]] calls) AND fixed the protocol's CDP auto-skip rule.

## ⚠️ NEXT-PM FIRST READ — open items requiring user direction

**1. T-309.3 unchanged.** Constitution-blocked under [[anti-cargo-cult-security]] hold per AGENTS.md P0 #5. Unblocks ONLY on real XSS / supply-chain incident, DOMPurify bypass, user explicit authorization, or external review requiring it. Path-constraint defense (`requireNonSymlinkWritePath` + `requireWriteWithinMemory`) bounds worst-case bypass to `~/maenifold/memory` regardless. Do NOT dispatch.

**2. T-326.4.1 SHIPPED post-close (~18:08 PT, after iter-14 was nominally closed).** KEY FINDING: the sampling wire format was ALREADY correct (`enable_sampling()` + `enable_sampling_tools()` serialize to `"sampling":{"tools":{}}` in the JSON-RPC initialize). T-303.4.21 v2/v3 failures were caused by a STALE daemon binary — even though I ran `launchctl kickstart -k gui/$UID/com.ma-collective.maenifold` twice mid-iter, the running daemon was apparently NOT replaced with the freshly-built one. Shipped: (1) wire-level integration test `initialize_request_wire_format_includes_sampling_capability`, (2) `tracing::debug!` diagnostic logging at handshake construction time so future UAT failures can grep daemon logs.

**3. T-303.4.21 is now dispatchable — but verify the daemon binary inode FIRST.** Before iter-15 dispatches T-303.4.21 v4, run:
```bash
cargo build --release -p maenifold-cli -p maenifold-pipeline-driver
launchctl kickstart -k gui/$UID/com.ma-collective.maenifold
sleep 5
# Verify the running daemon is actually the freshly-built one:
DAEMON_PID=$(launchctl list | grep com.ma-collective.maenifold | awk '{print $1}')
lsof -p $DAEMON_PID | grep txt | head -3
# Compare the inode/path to:
ls -i target/release/maenifold
# They MUST match. If they don't, the launchctl kickstart didn't replace the binary cleanly.
```
Also re-`npm run package` BEFORE dispatching CDP UAT (the packaged Electron binary is a separate copy).

## TL;DR — iter-14 result

- **23 shipped** — see `memory://mae-auto/iterations/iter1423shipsafteruserreopenedwontfixcascade`
- 4 new sprint-326 tasks filed AND dispatched same iter (NO PUNTING): T-326.2, T-326.3, T-326.4 (WONTFIX on cited evidence), T-326.4.1 (in flight)
- 2 sprint umbrellas closed (T-311.0, T-316.0 — all children done)
- 2 daemon redeploys + 1 Electron repackage mid-iter
- Protocol fix: CDP auto-skip rule REMOVED from `ralph/prompt.md` Cardinal Rule #1, "UAT priority floor" section, "Final note" + `ralph/AGENTS.md` "Programmatic UAT" bullet. CDP is now first-class product verification, dispatched the same as any other task. Only programmatic UAT (auto-generating per-TC tasks from `docs/UAT-PLAN.md`) remains banned.

## Backlog after iter close (9 in todo/)

**Constitution-blocked (1):**
- T-309.3 — fs/write permission renderer-trust hardening

**In flight (1):**
- T-326.4.1 — sampling capability wire-format fix (worktree alive; status uncertain per user; next-PM triages on boot)

**Blocked on T-326.4.1 (1):**
- T-303.4.21 — Phase 3 multi-turn sampling CDP UAT (fixture committed, evidence ready, just needs the wire fix)

**Mid-stage inherited from prior PMs (4 — eligible for dispatch any iter):**
- T-303.18.2 (swe) — CDP UAT: T-303.18.1 MCP prompts UI surface
- T-303.19.1 (swe) — UAT: T-303.19 MCP completion/complete renderer
- T-303.9 (execute) — mcp-apps UAT
- T-304.2 (swe) — Reconnect button CDP click-flow verify

These are pure CDP UAT tasks left in mid-pipeline by prior PMs. Per the new protocol they're normal-priority dispatchable — next PM should pull lowest-numbered first when capacity exists.

**Sprint umbrella awaiting last child (1):**
- T-304.0 — sprint 304 umbrella; only remaining child T-304.2 in todo/. Close after T-304.2 lands.

**Active in flight (already counted above):**
- T-326.4.1

## Recommendation for iter-15

1. **First action: check task-pm-t-326-4-1 status.** If still running 30+ min, kill, remove worktree, and either re-dispatch with explicit guidance per spec OR file decomposition (T-326.4.1.1 stderr-log + T-326.4.1.2 fix).
2. **If T-326.4.1 returns clean:** verify + merge + redeploy daemon + re-dispatch T-303.4.21 v4 (the fixture is committed, the recipe is in the v3 gate notes).
3. **Drain inherited mid-stage CDP UAT:** dispatch T-303.18.2, T-303.19.1, T-303.9, T-304.2 in parallel — they're independent, different files/components, no shared CDP port needed (each gets its own via uat-launch.sh).
4. **Close T-304.0 umbrella** once T-304.2 lands.

## Process state at iter close

- HEAD: should be the merge of T-316.0 + T-311.0 closures plus the iter-close commit.
- Worktrees: 1 (T-326.4.1). All others removed.
- Daemon: launchd-managed at PID resulting from second redeploy (last seen 98989 after 16:44 rebuild). Will likely have been restarted again by the time iter-15 boots if T-326.4.1 ships.
- Gate cache: synced 4+ times this iter (after T-322.1.7 file, T-303.4.21-v3 evidence commit, etc.)
- Memory note: written via WriteMemory at `memory://mae-auto/iterations/iter1423shipsafteruserreopenedwontfixcascade`
- Learnings: 5 new lessons appended to `ralph/state/learnings.md` (WONTFIX runtime-vs-compile-time, packaged Electron binary copy, Plyr/jsdom polyfills, worktree rebase before merge, user-reopened cascade re-evaluation rule)

## Skill / process improvements this iter

- Constitution: CDP-as-product-verification framing committed (`ralph/prompt.md` Cardinal Rule #1, UAT-floor section, Final note + `ralph/AGENTS.md` Programmatic UAT bullet). Future PMs will not treat CDP as auto-skip.
- 4 new sprint-326 fixes (config-discovery, ACP relay, sampling-relay foundation, sampling wire-format) all filed THIS iter and dispatched same iter per NO PUNTING — sprint 326 documents the daemon-side gap chain surfaced by T-303.4.21.

## Iter-14 throughput tally

- 24 tasks closed (22 SWE-driven ships + 2 umbrella closures, T-326.4.1 landed post-close)
- 1 task-pm escalation with mid-iter senior-remediation (T-302.12 via senior inner-loop worker)
- 4 new tasks filed this iter (T-316.2.1, T-322.1.7, T-326.4, T-326.4.1) — 3 shipped same iter
- 2 sprint-326 follow-ups filed (T-326.2, T-326.3) from T-303.4.21 escalations — both shipped same iter
- 12 gate_Reopen calls (the reversal that started the iter)
- 26 worker dispatches in total (task-pms + 1 senior inner-loop)
- 2 daemon redeploys (post T-326.2, post T-304.6.2)
- 1 Electron repackage (after all backend fixes)
- 2 merge conflicts resolved (acp_relay.rs T-326.3↔T-304.6.2; T-326.2 rebase artifact)
- Wall-clock: ~3.5 hours (14:30 → 18:05 PT)
