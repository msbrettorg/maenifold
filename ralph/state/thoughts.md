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

## Current Wave — T-03.5 and T-03.6 (CSS migration)
- Both scoped down from full library replacements to CSS→Tailwind migration
- T-03.6 (GraphViewCanvas): SWE done, red-team found P1 (dropped :slotted rule), remediated, re-auditing
- T-03.5 (PipelineComposer 13 files): Batch 1 done (4 files committed). Batch 2 SWE dispatched (6 files). Batch 3 remaining (PipelineFlyout, PipelineIconRail, PipelineSidebar)
- PipelineNodeCardPorts: ALL :deep() → audit-ok exemption (Vue Flow handles)
- PipelineComposer: retains :deep() audit-ok for Vue Flow Controls/MiniMap

## Next Actions
- Complete T-03.5 Batch 3 (3 files: Flyout, IconRail, Sidebar)
- Once T-03.5 + T-03.6 shipped → T-03.16 unblocked (all deps T-03.1–T-03.8 done)
- T-03.16 needs decomposition: ~75 remaining files with style blocks after pipeline/graph cleanup. Split by component group: Chat* (10), Gate* (8), Settings* (14), Workflow* (10), Thinking* (3), SemanticZoom* (4), TemplateGallery* (4), AgentProbe* (3), standalone (~19)
- T-204.0 epic (Handy speech-to-text) has 7 well-spec'd subtasks all pending — ready for dispatch when T-03 wave stabilizes