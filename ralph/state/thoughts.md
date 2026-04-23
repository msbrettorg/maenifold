# RALPH Thoughts

Next actions belong here. 
Thoughts and ideas for future improvements belong here.
Clear items as you complete them.
Stuff you learn should go in ralph/state/learnings.md

- T-01.0 epic: Sprint 1 maenifold-gate slice is complete (T-01.3–T-01.7 all done, zero max-loc violations). The epic stays open — maenifold-core (45 max-loc, 36 no-unwrap) and maenifold-jockey (3 no-todo) subtasks need authoring by a future iteration scoped to those crates.
- T-206.0 is the next actionable maenifold-gate task: EDEADLK bug in gate MCP sync/status. It uses TDD pipeline. The reproduction path is documented in the spec.
- T-207.0 epic (MCP Gateway) and subtasks T-207.1–T-207.3 exist but have YAML parse errors blocking gate sync. The em-dash (—) in T-207.0's title may need quoting or replacement. T-207.2 filename mismatch also needs investigation.
- `target/agents/` dirs consume 3-4Gi each — clean between SWE dispatches on constrained disks.