# RALPH Thoughts

Handoff from this iter to the next. Replace at iter end.

## Iter close 2026-05-16 (iter-6) — sprint 302 fully blocked on user decisions

**Working tree state at close:**
- Grooming-only commit pending — `ralph/state/learnings.md` (+2 lessons), `ralph/state/thoughts.md` (replaced), 3 spec frontmatter updates in todo/ (T-302.12, T-302.64, T-302.65; gitignored), T-301.40 closed via gate_Pass cascade (umbrella moved to done/ after all 3 children shipped in prior iters).
- No code changed. No worker dispatched. No worktree created.

## Sprint 302 — all remaining work is BLOCKED PENDING USER DECISIONS

After this iter's grooming, `gate_Status` shows sprint 302 with 3 tasks all `state: blocked, ready: false`. The blocked_reason field now surfaces the decision needed in `gate_Status` output (was previously buried in spec body — see iter-6 learning).

### T-302.65 (P1) — render_chart does not exist
- Decision required: (a) build a new chart-rendering Gateway tool (multi-iter feature) OR (b) rescope T-302.12 to test an existing visualization (e.g., maenifold.graph SVG output, ResourceLinkPreview inline image).
- Prior PM iter-5 explicitly held: "Hold for explicit user/PM decision; do NOT silently switch the scope without authorization."
- Iter-6 PM concurs. The user said "render_chart" by name on 2026-04-28 and we cannot ship without confirming the path forward. PM-grep verified zero matches across `crates/**`, `apps/desktop/src/**`, all 10 MCP servers in `~/maenifold/maenifold.json`.
- **Ask the user**, do not unilaterally choose.

### T-302.12 (P2) — depends_on T-302.65
- Just the CDP UAT shell. Cannot run until T-302.65 resolution lands.
- Now properly carries `depends_on: T-302.65` + `blocked: true` in frontmatter.

### T-302.64 (P2) — premise architecturally invalid
- Spec asks maenifold-jockey ACP adapters to emit ACP-compliant terminal/diff/in_progress shapes. PM grep confirms `crates/maenifold-jockey/src/acp/client.rs:124-183` is a pass-through CLIENT that forwards `SessionUpdate::ToolCall` from upstream agent CLIs (Codex, Claude, Copilot, OpenCode). The upstream CLIs OWN the shape emission; maenifold cannot fix their code.
- Three rescope options documented in the spec (A: file upstream issues, B: build a heuristic translation layer in jockey, C: accept fallback rendering + file upstream issues).
- **Recommended: Option C** (file upstream issues + accept fallback). Translation layer is a maintenance liability; upstream fix is the correct architecture.
- **Ask the user**, do not unilaterally choose.

## Daemon state at iter close

- **Daemon binary inode 316830419** matches disk (`target/release/maenifold`, May 15 17:11 build). Prior iter-5 thoughts.md INCORRECTLY warned the daemon was still on inode 315778576 — iter-6 verification (lsof) shows daemon (PID 8034) is running the freshest binary. **No daemon restart needed.** Carrying forward: the iter-5 PM mis-remembered the inode change; ALWAYS verify with `lsof -p $(pgrep -f 'target/release/maenifold' | head -1) | grep "txt.*release/maenifold$"` before believing any "daemon is stale" claim.
- Cleaned up: orphan packaged Electron from prior T-302.12 UAT (PID 15876, port 9473) killed.
- No active worktrees.
- No active workers.

## What did NOT happen this iter (and why)

- **No worker dispatched.** Sprint 302's 3 remaining tasks all require PM/user decisions; PM has no authority to unilaterally rescope user instructions or authorize new features (constitution P0 #5 analog).
- **No CDP UAT.** T-302.12 cannot UAT a phantom tool.
- **No daemon redeploy.** Daemon was already current.
- **No Electron repackage.** No frontend code changed.
- **No architect duo review.** Not a wave boundary (no umbrella close, no ship).
- **No move to sprint 303.** Prompt rule: "Do not mix sprints in iterations." Sprint 302 is the active sprint until all its work clears.

## Critical: iter-6 PM did NOT converge

Per `ralph/prompt.md`: "Do not converge if any pending or in-progress items remain, even if not in scope for you - end your turn and let the next PM pick them up." Sprint 302 has 3 blocked items — convergence marker NOT created.

## Suggested entry point for next loop

1. **Check if user has answered the two decision asks.** If the user has scrollback / a memory note / a recent comment on T-302.65 (build vs rescope) or T-302.64 (Option C vs A vs B), apply their decision:
   - T-302.65 build path → decompose into design + chart-lib selection + implementation + tests + integration UAT child tasks; ship over multiple iters.
   - T-302.65 rescope path → amend T-302.12 spec to target an existing visualization (suggest: maenifold.graph domain SVG render via Gateway), unblock T-302.12, run the CDP UAT.
   - T-302.64 Option C → file upstream issues against Codex/Claude/Copilot ACP CLIs for non-spec-compliant tool_call_update shapes; close T-302.64 as WONTFIX in maenifold's codebase with an `accepted-residual` rationale + memory note linking to the upstream tracking issues.
2. **If no user answer yet**, the sprint is fully blocked. Options:
   - Wait (do another iter and ask again).
   - PM author iter-7 spec amendment that ASKS THE USER explicitly with an `ask_user` style note in the iter handoff (escalation visibility).
3. **Do NOT** move to sprint 303 work until sprint 302 clears.

## Carry-forward open product gaps

- 16 LOC violations on baseline (per `bash scripts/audit-crates.sh`) — unchanged.
- handler_execute.rs has 2 .unwrap() violations (T-315.1/T-315.2 backlog).
- 9 sprint-302 product defects unshipped (T-302.50, .53, .55, .57-.61, .63 — tool-call rendering cluster). These are NOT in sprint-302 scope because they were filed against the sprint after this iter started; they're future-sprint cleanup. Verify by re-grep before treating as blocking.

## Files modified this iter

- `done/T-301.40.md` (moved from `todo/` via gate_Pass cascade — umbrella close, all 3 children shipped)
- `todo/T-302.12-244.97-render-chart-ui-test.md` (frontmatter: +depends_on, +blocked, +blocked_reason)
- `todo/T-302.64-agent-acp-adapter-terminal-content-emission.md` (frontmatter: +blocked, +blocked_reason; body: +2026-05-16 premise reassessment with 3 rescope options)
- `todo/T-302.65-render-chart-gateway-registration.md` (frontmatter: +blocked, +blocked_reason)
- `ralph/state/learnings.md` (+2 H3 lessons: "PM-decision-required tasks must be marked blocked", "render_chart phantom feature")
- `ralph/state/thoughts.md` (replaced)

## Iter throughput tally

- **0 tasks shipped** (sprint 302 is fully blocked on user decisions)
- **1 task closed via grooming** (T-301.40 superseded umbrella → done; 4 gate_Pass calls)
- **3 spec frontmatters corrected** (T-302.12, .64, .65 now correctly show as blocked in gate_Status)
- **2 durable lessons appended** to learnings.md
- **1 orphan Electron process** reaped (PID 15876 from prior T-302.12 UAT)
- **0 worktrees** created
- **0 workers** dispatched
- **0 rubber-duck reviews** run (no wave boundary)
- **0 daemon rebuilds** (already current)
- **0 Electron repackages** (no UI changed)

This iter was honest grooming + truthful block-state capture. The next iter cannot ship sprint-302 work without user input on the two decision items above.
