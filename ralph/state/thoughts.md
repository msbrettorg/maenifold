# RALPH Thoughts

Next actions belong here. 
Thoughts and ideas for future improvements belong here.
Clear items as you complete them.
Stuff you learn should go in ralph/state/learnings.md

- T-02.x UI audit epic in progress. Waves 1+2 done (8 tasks: T-02.9–T-02.12, T-02.15–T-02.16, T-02.18, T-02.20).
- Wave 3 in flight: T-02.13 (external CSS — SWE done, red-team auditing), T-02.1 batch 1 of 5 files (SWE running)
- T-02.6 spec FIXED — original had wrong Tailwind v4 radius mappings. Also found --radius-8 bug (28 undefined var refs). Correct: rounded-lg=4px, rounded-xl=8px in this config.
- T-02.2-T-02.5 spec CSS rule counts are inflated (said 82-108, actual 15-36) — specs counted properties not selectors
- T-02.7/T-02.8/T-02.19 (prop drilling) are complex refactors — hold for later wave
- T-205.705 closed — redundant View All Agents button, UAT TC-039 marked N/A
- T-207.0 epic (MCP Gateway) and subtasks T-207.1–T-207.9 parse correctly. All at pending.
- T-203.0 epic (MCP Apps Host Support) and subtasks T-203.1–T-203.7 all at pending.
- T-205.705: pending in test pipeline. UI task — "View all agents" button. Needs decision: implement or cancel.
- `target/agents/` dirs consume 3-4Gi each — clean between SWE dispatches on constrained disks.