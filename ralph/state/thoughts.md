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

## Wave 2 DONE — Wave 3 Ready
T-03.2, T-03.3, T-03.4, T-03.7 all shipped. T-03.8 SWE dispatched (style migration for AgentSelector, ConfigOptionPicker, WorkflowStepTimeline).
- Pre-installed deps in main tree before worktrees to avoid package.json merge conflicts
- vue-json-pretty, markdown-it, @shikijs/markdown-it, @tanstack/vue-table, shadcn table+stepper all landed
- WikiLink pills in KnowledgeNoteRenderer now via markdown-it inline plugin (html:false safe)
- ThinkingMarkdownContent.vue deleted (consolidated into KnowledgeNoteRenderer with clamp prop)
- PanelResizeHandle.vue deleted (WorkflowAuthoringPage uses shadcn ResizablePanelGroup)
- T-03.4 spec was stale (SchemaForm.css didn't exist, prop counts wrong) — grounded and simplified to just SchemaFormRawEditor.vue style migration
- T-03.8 spec updated: EmptyState dropped (already clean), WorkflowStepTimeline scoped to CSS migration not component replacement (it's a timeline, not a stepper)
- Next: T-03.9/T-03.10 prop drilling, T-03.16 final sweep

## Backlog Notes
- T-207.0 epic (MCP Gateway) and subtasks T-207.1–T-207.9 all at pending
- T-203.0 epic (MCP Apps Host Support) and subtasks T-203.1–T-203.7 all at pending
- `target/agents/` dirs consume 3-4Gi each — clean between SWE dispatches on constrained disks