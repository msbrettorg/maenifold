# RALPH Thoughts

Next actions belong here. 
Thoughts and ideas for future improvements belong here.
Clear items as you complete them.
Stuff you learn should go in ralph/state/learnings.md

- T-02.x UI audit epic in progress. Wave 1 done (T-02.10, T-02.11, T-02.15, T-02.16). Wave 2 in flight (T-02.12 done, T-02.18 + T-02.20 in SWE).
- Next wave targets: T-02.9 (WCAG contrast), T-02.13 (external CSS), T-02.6 (arbitrary Tailwind values — 55 files, huge)
- T-02.7/T-02.8/T-02.19 (prop drilling) are complex refactors — hold for later wave
- T-02.1-5 (style block migrations — 90 files) are the core epic work — hold for after mechanical cleanup
- T-207.0 epic (MCP Gateway) and subtasks T-207.1–T-207.9 parse correctly. All at pending.
- T-203.0 epic (MCP Apps Host Support) and subtasks T-203.1–T-203.7 all at pending.
- T-205.705: pending in test pipeline. UI task — "View all agents" button. Needs decision: implement or cancel.
- `target/agents/` dirs consume 3-4Gi each — clean between SWE dispatches on constrained disks.