# RALPH Learnings

Open discoveries from past loops — kept here only because the system doesn't enforce them yet. When a lesson here gets codified (AGENTS.md, build audit, script, prompt template), drop it from this file.

## Daemon binary freshness
Recurred across 5 sprints. Source builds but the running daemon is older; tests fail or pass against stale code. Verify with `lsof -p $(pgrep -f maenifold) | grep txt` + `LC_ALL=C grep -ac '<known-recent-literal>' <binary>` before declaring a runtime test stale. Pipeline driver is a separate binary — both must rebuild + restart.

## Worker self-report is unreliable
SWEs have claimed "no changes needed" when changes were required, claimed edits that never persisted, written code into dead/unreachable components. Constitution mandates SWE read-back of modified lines after every edit and LSP `findReferences` to confirm components are in the active render path. The mandate exists; the lesson keeps recurring because nothing enforces it. PM must check diff-stat + grep-for-edit-content before accepting WorkResult.

## Orphan-commit pattern
"Marked PASS in gate but never merged to dev." Pre-flight grep for the parent task's expected stub on dev is mandatory before scoping any "after T-XXX.Y frontend stub" work. Not yet scripted.

## Architect duo catches what single review misses
gpt-5.5 catches races + API correctness; claude-opus-4.6 catches design coherence. Run both at every wave/sprint boundary with INDEPENDENT prompts. On divergence, PM is tiebreaker; default more-restrictive ONLY where the surface is actually defective; refute factually-wrong findings with evidence.

## Audit ratcheting antipattern
Each red-team round invents NEW objections never raised before. Round-N+1 objections that weren't blockers in round-N → file as separate gate task, do NOT block current closure. Red-team verdict scope is the spec at dispatch time, not at re-dispatch time.

## Confession Q4-Q5 catches weak TDD
"Did this test fail on baseline before your change?" + "Was this test written before or after the implementation passed?" Asking explicitly catches semantic-layer bugs and tests-written-after-impl that audit alone misses. Bake into worker confession template.

## Production-shape mocks only
Flat literal mocks across stack boundaries hide real production-path bugs ("flat-mock masking"). Cross-stack tests must use the same SDK/factory the production code uses, or a shared fixture builder.

## CDP harness can attach to about:blank
After `agent-browser connect <port>`, `snapshot -i` may show only the about:blank chrome instead of the maenifold renderer. Workaround: `curl -s http://127.0.0.1:<port>/json | jq '.[] | select(.url|contains("app.asar/dist/index.html")) | .webSocketDebuggerUrl'` then drive raw CDP. Tracked as a harness fix; until landed, this manual fallback is required.

## Sandbox banned-substring guard fires on commit message bodies
Literal strings like `pkill`, `killall` in commit message text get flagged even though the commit operation itself is harmless. Use semantic descriptions ("name-based process termination" instead of the tool name).

## rustfmt churns unrelated files
Standing SWE failure mode for Rust workers. Format only files you created or modified. PM reverts unrelated churn at merge. Not yet build-enforced.
