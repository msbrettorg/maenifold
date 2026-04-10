# Requirements Traceability Matrix

| Requirement | Component | Test Case | Type |
|-------------|-----------|-----------|------|
| FR-1 | src/App.vue | TC-1: design tokens defined in :root | typecheck |
| FR-1 | src/App.vue | TC-2: dark mode overrides present | typecheck |
| FR-2 | src/components/PipelineRunner.vue | TC-3: error banner renders on MCP failure | typecheck |
| FR-2 | src/components/PipelineInspector.vue | TC-4: action errors surfaced | typecheck |
| FR-3 | src/components/PipelineNodeCard.vue | TC-5: no hardcoded #fff in color-mix | typecheck |
| FR-3 | src/App.vue | TC-6: --bg-panel defined | typecheck |
| FR-3 | src/components/PipelineRunner.vue | TC-7: status colors use CSS classes | typecheck |
| FR-4 | src/lib/pipeline-types.ts | TC-8: ToolNode in discriminated union | typecheck |
| FR-4 | src/components/AddNodeForm.vue | TC-9: tool node form fields | typecheck |
| FR-4 | src/components/PipelineNodeCard.vue | TC-10: tool node color and icon | typecheck |
| FR-4 | crates/maenifold-core/src/workflow/pipeline/types.rs | TC-11: Tool variant in PipelineNode enum | unit |
| FR-4 | crates/maenifold-core/src/workflow/pipeline/executor.rs | TC-12: tool node execution | unit |
| FR-5 | src/stores/pipeline.ts | TC-13: fork/join/dispatch exported | typecheck |
| FR-5 | src/composables/usePipelineMcp.ts | TC-14: fork includes newSessionId | typecheck |
| FR-6 | src/App.vue | TC-15: shared .btn classes defined | typecheck |
| FR-6 | src/components/PipelineInspector.vue | TC-16: busy state prevents double-click | typecheck |
| FR-7 | src/App.vue | TC-17: shared .field-input classes defined | typecheck |
| FR-7 | src/App.vue | TC-18: :focus-visible ring on inputs | typecheck |
| FR-8 | crates/maenifold-core/src/workflow/pipeline/types.rs | TC-19: agent field on all node types | unit |
| FR-8 | crates/maenifold-core/src/workflow/pipeline/types.rs | TC-20: acpClient backwards compat | unit |
| FR-9 | crates/maenifold-core/src/workflow/pipeline/engine.rs | TC-21: abandoned nodes → terminal status | unit |
| FR-10 | crates/maenifold-core/src/workflow/pipeline/view.rs | TC-22: view includes all node fields | unit |
| FR-11 | crates/maenifold-core/src/workflow/pipeline/handler.rs | TC-23: rejects unknown agent at creation | unit |
| FR-12 | crates/maenifold-core/src/workflow/pipeline/dispatch.rs | TC-24: type field in dispatch response | unit |
| NFR-1 | src/components/ | TC-25: focus-visible on all interactive | typecheck |
| NFR-1 | src/components/PipelineRunner.vue | TC-26: aria-label on expand button | typecheck |
| NFR-2 | src/components/PipelineComposer.vue | TC-27: no deep watch on localDefinition | typecheck |
| NFR-2 | src/components/PipelineRunner.vue | TC-28: mermaid.initialize called once | typecheck |
| NFR-3 | src/App.vue | TC-29: responsive breakpoint at 900px | typecheck |
| NFR-4 | src/App.vue | TC-30: 5-step type scale tokens | typecheck |
| NFR-5 | src/composables/acp-bridge.ts | TC-31: no console.log in production | typecheck |
| FR-13 | crates/maenifold-core/src/workflow/pipeline/types.rs | TC-32: Tool variant agent field is Option with no default | unit |
| FR-13 | crates/maenifold-core/src/workflow/pipeline/types.rs | TC-33: Tool node with agent field deserializes (backward compat) | unit |
| FR-13 | crates/maenifold-core/src/workflow/pipeline/types.rs | TC-34: Tool node without agent field deserializes | unit |
| FR-13 | crates/maenifold-core/src/workflow/pipeline/executor.rs | TC-35: Tool node executes without ACP client configured | integration |
| FR-13 | crates/maenifold-core/src/workflow/pipeline/executor.rs | TC-36: Tool node does not check agent field during execution | integration |
| FR-13 | crates/maenifold-core/src/workflow/pipeline/mod.rs | TC-37: Tool node creation does not validate agent | integration |
| FR-14 | src/lib/pipeline-types.ts | TC-38: ToolNode interface has optional agent | typecheck |
| FR-14 | src/components/AddNodeForm.vue | TC-39: no agent dropdown rendered for tool node type | typecheck |
| FR-14 | src/components/PipelineInspector.vue | TC-40: no agent field in tool node inspector | typecheck |
| FR-14 | src/components/PipelineNodeCard.vue | TC-41: no agent pill on tool node card | typecheck |
| FR-14 | src/composables/useAddNodeForm.ts | TC-42: toolAgent removed from composable | typecheck |
| FR-15 | crates/maenifold-core/src/workflow/pipeline/types.rs | TC-43: PipelineEdge has optional mapping field | unit |
| FR-15 | crates/maenifold-core/src/workflow/pipeline/types.rs | TC-44: edge serializes without mapping field when None | unit |
| FR-15 | crates/maenifold-core/src/workflow/pipeline/types.rs | TC-45: edge deserializes with mapping field present | unit |
| FR-15 | crates/maenifold-core/src/workflow/pipeline/types.rs | TC-46: edge deserializes without mapping field (backward compat) | unit |
| FR-16 | crates/maenifold-core/src/workflow/pipeline/executor.rs | TC-47: tool node receives mapped args from completed source | integration |
| FR-16 | crates/maenifold-core/src/workflow/pipeline/executor.rs | TC-48: mapped values override static tool_args | integration |
| FR-16 | crates/maenifold-core/src/workflow/pipeline/executor.rs | TC-49: unresolved dot-path produces null value | integration |
| FR-16 | crates/maenifold-core/src/workflow/pipeline/executor.rs | TC-50: edge without mapping uses static args unchanged | integration |
| FR-16 | crates/maenifold-core/src/workflow/pipeline/executor.rs | TC-51: multiple incoming edges with mappings all resolved | integration |
| FR-16 | crates/maenifold-core/src/workflow/pipeline/executor.rs | TC-52: dot-path resolves nested JSON (results.0.path) | integration |
| FR-17 | src/lib/pipeline-types.ts | TC-53: PipelineEdge has optional mappings field | typecheck |
| FR-17 | src/components/EdgeMappingEditor.vue | TC-54: editor renders target input params from tool schema | typecheck |
| FR-17 | src/components/EdgeMappingEditor.vue | TC-55: editor handles tools-not-loaded empty state | typecheck |
| FR-17 | src/components/EdgeMappingEditor.vue | TC-56: editor handles no-source-output empty state | typecheck |
| FR-17 | src/components/EdgeMappingEditor.vue | TC-57: editor handles no-mappings-configured empty state | typecheck |
| FR-17 | src/components/EdgeMappingEditor.vue | TC-58: ACP target shows informational message | typecheck |
| FR-17 | src/components/PipelineInspector.vue | TC-59: edge mapping editor integrated in edge section | typecheck |
| FR-18 | src/components/PipelineComposer.vue | TC-60: edge with mapping shows data-flow badge | typecheck |
| FR-18 | src/components/PipelineComposer.vue | TC-61: tool edge without mapping shows warning indicator | typecheck |
| FR-18 | src/components/PipelineComposer.vue | TC-62: edge hover displays mapping summary | typecheck |
| FR-19 | src/components/PipelineInspectorServer.vue | TC-63: server inspector shows source output | typecheck |
| FR-19 | src/components/PipelineInspectorServer.vue | TC-64: server inspector shows mapping config | typecheck |
| FR-19 | src/components/PipelineInspectorServer.vue | TC-65: server inspector shows resolved target input | typecheck |
| FR-20 | crates/maenifold-core/src/workflow/pipeline/view.rs | TC-66: view includes assembled_prompt for completed Prompt node | integration |
| FR-20 | crates/maenifold-core/src/workflow/pipeline/view.rs | TC-67: view includes assembled_prompt for completed Workflow node | integration |
| FR-20 | crates/maenifold-core/src/workflow/pipeline/view.rs | TC-68: view includes assembled_prompt for completed ST node | integration |
| FR-20 | crates/maenifold-core/src/workflow/pipeline/view.rs | TC-69: assembled_prompt absent for pending/active nodes | integration |
| FR-20 | crates/maenifold-core/src/workflow/pipeline/view.rs | TC-70: assembled_prompt reconstructs predecessor + ambient + instruction | integration |
| FR-21 | crates/maenifold-core/src/workflow/pipeline/view.rs | TC-71: view includes thinking_trace array for ST node | integration |
| FR-21 | crates/maenifold-core/src/workflow/pipeline/view.rs | TC-72: thinking_trace elements have thought_number, text, status | integration |
| FR-21 | crates/maenifold-core/src/workflow/pipeline/view.rs | TC-73: thinking_trace reads from session data via session_id | integration |
| FR-21 | crates/maenifold-core/src/workflow/pipeline/view.rs | TC-74: thinking_trace absent for non-ST nodes | integration |
| FR-22 | crates/maenifold-core/src/workflow/pipeline/view.rs | TC-75: view includes workflow_steps array for Workflow node | integration |
| FR-22 | crates/maenifold-core/src/workflow/pipeline/view.rs | TC-76: workflow_steps elements have step_name, status, output | integration |
| FR-22 | crates/maenifold-core/src/workflow/pipeline/view.rs | TC-77: workflow_steps absent for non-Workflow nodes | integration |
| FR-23 | src/components/PromptAssemblyPreview.vue | TC-78: renders predecessor context section | typecheck |
| FR-23 | src/components/PromptAssemblyPreview.vue | TC-79: renders ambient prompt section | typecheck |
| FR-23 | src/components/PromptAssemblyPreview.vue | TC-80: renders node instruction section | typecheck |
| FR-23 | src/components/PromptAssemblyPreview.vue | TC-81: sections are collapsible | typecheck |
| FR-23 | src/components/PromptAssemblyPreview.vue | TC-82: component absent when assembled_prompt is null | typecheck |
| FR-23 | src/components/PipelineInspectorServer.vue | TC-83: prompt preview integrated for Prompt nodes | typecheck |
| FR-23 | src/components/PipelineInspectorServer.vue | TC-84: prompt preview integrated for Workflow nodes | typecheck |
| FR-23 | src/components/PipelineInspectorServer.vue | TC-85: prompt preview integrated for ST nodes | typecheck |
| FR-24 | src/components/ThinkingTraceView.vue | TC-86: renders thoughts as numbered cards | typecheck |
| FR-24 | src/components/ThinkingTraceView.vue | TC-87: per-thought status indicators present | typecheck |
| FR-24 | src/components/ThinkingTraceView.vue | TC-88: revision markers displayed | typecheck |
| FR-24 | src/components/ThinkingTraceView.vue | TC-89: component absent when thinking_trace is null | typecheck |
| FR-24 | src/components/PipelineInspectorServer.vue | TC-90: thinking trace integrated for ST nodes | typecheck |
| FR-25 | src/components/WorkflowStepProgress.vue | TC-91: renders steps as vertical timeline | typecheck |
| FR-25 | src/components/WorkflowStepProgress.vue | TC-92: per-step status indicators present | typecheck |
| FR-25 | src/components/WorkflowStepProgress.vue | TC-93: current step highlighted | typecheck |
| FR-25 | src/components/WorkflowStepProgress.vue | TC-94: step output displayed | typecheck |
| FR-25 | src/components/WorkflowStepProgress.vue | TC-95: component absent when workflow_steps is null | typecheck |
| FR-25 | src/components/PipelineInspectorServer.vue | TC-96: workflow progress integrated for Workflow nodes | typecheck |
| FR-26 | crates/maenifold-core/src/workflow/pipeline/handler.rs | TC-97: fork copies predecessor output to forked root | integration |
| FR-26 | crates/maenifold-core/src/workflow/pipeline/handler.rs | TC-98: forked root assembles prompt with inherited context | integration |
| FR-26 | crates/maenifold-core/src/workflow/pipeline/handler.rs | TC-99: fork at first node has no predecessor context | integration |
| FR-27 | src/stores/pipeline.ts | TC-100: cycle detection prevents create | typecheck |
| FR-27 | src/stores/pipeline.ts | TC-101: unreachable node detection | typecheck |
| FR-27 | src/stores/pipeline.ts | TC-102: dangling edge detection | typecheck |
| NFR-6 | src/components/ | TC-103: new T-64 components under 300 LOC | review |
| NFR-7 | crates/maenifold-core/src/workflow/pipeline/executor.rs | TC-104: no auto-mapping on missing edge mapping | integration |
| NFR-7 | crates/maenifold-core/src/workflow/pipeline/executor.rs | TC-105: no type coercion on mapped values | integration |
| NFR-7 | src/components/ | TC-106: errors preserved, not auto-dismissed | typecheck |
| NFR-8 | crates/maenifold-core/src/workflow/pipeline/ | TC-107: BE specs have integration tests not just cargo check | review |
| NFR-8 | src/ | TC-108: FE behavioral reqs have component tests | review |
| FR-28 | src/styles/design-tokens.css | TC-109: surface hierarchy tokens defined (#131313 through #353534) | typecheck |
| FR-28 | src/styles/*.css, src/components/*.vue | TC-110: zero 1px solid structural borders remain | review |
| FR-29 | src/styles/fonts.css | TC-111: @font-face for Space Grotesk, Inter, JetBrains Mono | typecheck |
| FR-29 | src/styles/design-tokens.css | TC-112: --font-headline, --font-sans, --font-mono tokens reference bundled fonts | typecheck |
| FR-30 | src/styles/design-tokens.css | TC-113: no @media prefers-color-scheme light/dark blocks | typecheck |
| FR-31 | src/styles/glass.css | TC-114: .glass-surface class with backdrop-filter | typecheck |
| FR-31 | src/styles/design-tokens.css | TC-115: --ghost-border-color token defined | typecheck |
| FR-32 | src/components/PipelineNodeCard.vue | TC-116: node card uses surface_container_highest, 4px left light-bar | typecheck |
| FR-32 | src/components/PipelineNodeCard.vue | TC-117: no border declarations except left accent | typecheck |
| FR-33 | src/styles/design-tokens.css | TC-118: --shadow-ambient token defined, no hard drop shadows | typecheck |
| FR-34 | src/stores/semantic-zoom.ts | TC-119: zoom level state machine (graph/inspect/session) | typecheck |
| FR-34 | src/stores/semantic-zoom.ts | TC-120: enterInspect/enterSession/back actions | typecheck |
| FR-35 | src/composables/useFlipAnimation.ts | TC-121: FLIP animation composable with captureSource/animateToTarget | typecheck |
| FR-35 | src/composables/useComposerCanvas.ts | TC-122: onNodeDoubleClick triggers enterInspect | typecheck |
| FR-35 | src/components/PipelineComposer.vue | TC-123: .zoom-dimmed CSS class defined | typecheck |
| FR-36 | src/components/SemanticZoomOverlay.vue | TC-124: L1 overlay with header, predecessor context, output sections | typecheck |
| FR-36 | src/components/SemanticZoomOverlay.vue | TC-125: "Enter Session" button only for ACP nodes with sessionId | typecheck |
| FR-37 | src/components/SemanticZoomOverlay.vue | TC-126: L2 mounts ChatView bound to node sessionId | typecheck |
| FR-37 | src/components/PipelineBreadcrumb.vue | TC-127: breadcrumb shows DAG position with navigable pills | typecheck |
| FR-38 | src/composables/useEdgeParticles.ts | TC-128: edge particle composable with startParticles/stopParticles | typecheck |
| FR-39 | src/styles/node-animations.css | TC-129: .node-ready-pulse, .node-active-glow, .node-completion-ripple | typecheck |
| FR-39 | src/components/PipelineNodeCard.vue | TC-130: animation classes applied based on node status | typecheck |
| NFR-9 | src/composables/useFlipAnimation.ts | TC-131: animation uses cubic-bezier(0.4,0,0.2,1), 350ms duration | typecheck |
| NFR-10 | src/composables/useEdgeParticles.ts | TC-132: single rAF loop, imperative SVG updates | review |

## Dependency Map

| Spec | Depends On | Reason |
|------|-----------|--------|
| T-64.1 | T-64.0 | FE types mirror BE types |
| T-64.2 | T-64.0 | Both modify executor.rs tool dispatch |
| T-64.3 | T-64.2 | FE mapping needs BE mapping type |
| T-64.4 | T-64.3 | Visual indicators need mapping data |
| T-64.5 | T-64.0 | View refactor needs clean tool node type |
| T-64.6 | T-64.5 | FE prompt preview needs BE assembled_prompt |
| T-64.7 | T-64.5 | FE thinking view needs BE thinking_trace |
| T-64.8 | T-64.5 | FE workflow view needs BE workflow_steps |
| T-64.11 | T-64.2 | Fork context follows edge mapping pattern |
| T-67.4 | T-67.2, T-67.3 | Light mode retirement after dark tokens finalized |
| T-67.5 | T-67.2, T-67.4 | Border elimination needs tokens and dark-only mode |
| T-67.6 | T-67.5 | Pipeline component borders after CSS file borders |
| T-67.7 | T-67.5 | Misc component borders after CSS file borders |
| T-67.8 | T-67.2 | Shadow conversion needs surface tokens |
| T-67.9 | T-67.2, T-67.3 | Color cleanup needs token values finalized |
| T-67.10 | T-67.1, T-67.2, T-67.3 | Node card redesign needs fonts + tokens |
| T-67.11 | T-67.2 | Glassmorphism needs surface_variant token |
| T-67.14 | T-67.12 | Double-click wiring needs zoom store |
| T-67.16 | T-67.11, T-67.12, T-67.13 | L1 overlay needs glass, store, FLIP |
| T-67.17 | T-67.16 | L2 session dive extends L1 overlay |
| T-67.18 | T-67.12, T-67.10 | Keyboard nav needs store, animations need node card |

## Cross-Reference: Spec to FR

| Spec | Requirements |
|------|-------------|
| T-64.0 | FR-13 |
| T-64.1 | FR-14 |
| T-64.2 | FR-15, FR-16 |
| T-64.3 | FR-17 |
| T-64.4 | FR-18, FR-19 |
| T-64.5 | FR-20, FR-21, FR-22 |
| T-64.6 | FR-23 |
| T-64.7 | FR-24 |
| T-64.8 | FR-25 |
| T-64.11 | FR-26 |
| T-67.1 | FR-29 |
| T-67.2 | FR-28 |
| T-67.3 | FR-28, FR-33, NFR-9 |
| T-67.4 | FR-30 |
| T-67.5 | FR-28 |
| T-67.6 | FR-28 |
| T-67.7 | FR-28 |
| T-67.8 | FR-33 |
| T-67.9 | FR-28 |
| T-67.10 | FR-32 |
| T-67.11 | FR-31 |
| T-67.12 | FR-34 |
| T-67.13 | FR-35, NFR-9 |
| T-67.14 | FR-35 |
| T-67.15 | FR-38, NFR-10 |
| T-67.16 | FR-35, FR-36 |
| T-67.17 | FR-37 |
| T-67.18 | FR-34, FR-39 |
| T-68.1 | FR-40, FR-41 |
