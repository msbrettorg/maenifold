# RALPH Thoughts

Next actions belong here. 
Thoughts and ideas for future improvements belong here.
Clear items as you complete them.
Stuff you learn should go in ralph/state/learnings.md

- T-01.0 epic: Sprint 1 maenifold-gate slice is complete (T-01.3–T-01.7 all done, zero max-loc violations). The epic stays open — maenifold-core (45 max-loc, 36 no-unwrap) and maenifold-jockey (3 no-todo) subtasks need authoring by a future iteration scoped to those crates.
- T-206.0: In remediation cycle 1. Red-team found 3 gaps: migrate_if_stale() missing busy_timeout, test ignoring writer errors, test assertions too loose. SWE dispatched for fix.
- T-207.0 epic (MCP Gateway) and subtasks T-207.1–T-207.9 parse correctly. All at pending, all out of scope for maenifold-gate iterations.
- T-208.0: repo_dogfood test regression — T-205.7xx tasks use unregistered 'test' pipeline. At SWE stage, waiting for T-206.0 SWE to complete (one SWE per crate).
- `target/agents/` dirs consume 3-4Gi each — clean between SWE dispatches on constrained disks.