# RALPH Thoughts

Next actions belong here. 
Thoughts and ideas for future improvements belong here.
Clear items as you complete them.
Stuff you learn should go in ralph/state/learnings.md

## T-03 Zero Handrolled Epic — COMPLETE
- T-03.0 epic: all 25 subtasks (T-03.1–T-03.25) shipped
- 90+ components migrated from `<style scoped>` to Tailwind
- ~6,500 CSS lines removed
- 32 files retain audit-ok exempted style blocks
- T-03.26 filed for future: strip ALL audit-ok exemptions

## Wave 2 and Wave 3 DONE — T-03 Epic Complete
- All subtasks shipped through 3 waves of parallel SWE dispatch
- Pre-installed deps in main tree before worktrees to avoid package.json merge conflicts

## Current Wave — T-03 Epic Complete
- All subtasks shipped. T-03.0 epic can be closed.
- T-03.26 filed: strip ALL audit-ok exemptions and ban at build time

## Next Actions
- T-05.87, T-05.92, T-05.94, T-05.95 ready for dispatch in next iteration
- T-05.67 (vec0 wizard) is P2 — defer until P1s clear
- prewarm.rs config error swallowing filed as informational by red-team (T-05.91) — needs its own task
- Agents filing tasks during blue-team (T-05.85-95) is a process violation — blue-team should only verify, not file. But findings are real. Need to add explicit "do NOT file tasks" to blue-team prompts.
- Gate database locking should be better now with T-05.84 mutex — verify in next iteration
## Sprint 05 — Component Rebuild (in progress)
- 56 components deleted and rebuilt from scratch — all pass rejection criteria
- Codex audit found 47 MORE files with hand-rolled patterns (ScrollArea, Card, Skeleton, Dialog, Progress, Badge, Tooltip)
- 12 gate tasks filed (T-05.72–T-05.83) for the remaining hand-rolled violations
- Key bugs fixed this sprint: ConfirmDialog race condition, PipelineView CSS Grid JIT failure, gate MCP title parsing, Electron CSP for 127.0.0.1
- Removed Vue Flow MiniMap + Controls (unthemeable without custom CSS)
- Pipeline modal dialogs widened to 80vw
- Runner panel defaults collapsed
- DONE: T-05.71 (shadcn imports 18 files), T-05.72 (AppSidebar ScrollArea), T-05.81 (Card containers 9 files), T-05.82 (ScrollArea 11 files)
- IN-FLIGHT: T-05.69 (title parsing fix), T-05.70 (column widths fix), T-05.73-75 (SWE wave)
- Red-team caught: empty title fallback (T-05.69), column width mismatch (T-05.70) — both in remediation
- PENDING: T-05.66, T-05.67, T-05.76-80, T-05.83
- OVERLAP WARNING: T-05.77 + T-05.80 share WorkflowStepDetail.vue; T-05.79 + T-05.80 share WorkflowStepProgress.vue — sequence these
- NEED: build enforcement to PREVENT hand-rolled components from being introduced — audit-components.mjs needs `require-design-import` rule
- NEED: the audit from codex should be turned into a build rule, not just a one-time scan
- LEARNING: gate MCP "database is locked" errors are intermittent but may succeed — always verify status after. Double-pass race conditions have advanced tasks past intended stages.
