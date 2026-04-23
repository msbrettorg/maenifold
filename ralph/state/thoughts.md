# RALPH Thoughts

Next actions belong here. 
Thoughts and ideas for future improvements belong here.
Clear items as you complete them.
Stuff you learn should go in ralph/state/learnings.md

## T-03 Zero Handrolled Epic (in progress)
- T-03.0 epic with 16 subtasks, executing in waves
- T-03.12 DONE — dead code already removed in prior sprints
- T-03.1 (token infra P0) — SWE done, red-team auditing. 78 lines added to @theme: status, node, utility tokens + 39 color-mix precomputed blends
- T-03.13 (electron security P0) — SWE done, red-team FAILED: prefix-matching bypass in will-navigate + overly broad file: URL allowance. Remediation dispatched.
- T-03.14 (memory leaks) — DONE. onScopeDispose for interval/drag cleanup, teardown() for EventSource.
- T-03.15 (error handling) — DONE. Red-team caught P1 (sendPrompt swallowing errors → silent input loss). Fixed: returns boolean, ChatView/App.vue check before clearing input.

## Wave 2 Ready
T-03.2 through T-03.8 are component replacements unblocked by T-03.1 token infrastructure. Each is a substantial refactor requiring library installs + component rewrites. Next iteration should start these.
- 105 components have `<style>` blocks — T-03.2 through T-03.8 will replace hand-rolled components after T-03.1 lands
- 130+ distinct color-mix patterns in components — ~60 blend-to-transparent (use Tailwind opacity), ~70 two-color blends (pre-computed tokens or per-component migration)
- T-03.9/T-03.10 prop drilling, T-03.11 WCAG contrast, T-03.16 final sweep all blocked on T-03.1

## Backlog Notes
- T-207.0 epic (MCP Gateway) and subtasks T-207.1–T-207.9 all at pending
- T-203.0 epic (MCP Apps Host Support) and subtasks T-203.1–T-203.7 all at pending
- `target/agents/` dirs consume 3-4Gi each — clean between SWE dispatches on constrained disks