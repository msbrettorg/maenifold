<script setup lang="ts">
import { Button } from '@/components/ui/button'
import InspectorNodeActions from './InspectorNodeActions.vue'
import PipelineInspectorServerNodeTechnical from './PipelineInspectorServerNodeTechnical.vue'
import type {
  PromptViewNode,
  SequentialThinkingViewNode,
  ToolViewNode,
  ViewNode,
  WorkflowViewNode,
} from '../lib/pipeline-types'

defineProps<{
  selectedViewNode: ViewNode
  selectedViewNodeLabel: string
  selectedNodeHeadline: string
  selectedNodeOutput: string
  selectedNodeAgentSummary: string | null
  selectedNodeReconnectSessionId: string | null
  selectedNodeCanReconnect: boolean
  openingAgents: boolean
  showToolDirectExecutionExplanation: boolean
  showExpiredSessionExplanation: boolean
  shouldShowExpertDetails: boolean
  selectedToolResolvedArgs: Record<string, unknown> | null
  selectedWorkflowViewNode: WorkflowViewNode | null
  selectedPromptViewNode: PromptViewNode | null
  selectedThinkingViewNode: SequentialThinkingViewNode | null
  selectedToolViewNode: ToolViewNode | null
}>()

const emit = defineEmits<{
  (e: 'open-agents'): void
  (e: 'copy', value: string): void
}>()

function formatNodeType(type?: ViewNode['type']): string {
  return type === 'sequentialThinking' ? 'Sequential Thinking' : type === 'prompt' ? 'Prompt' : type === 'tool' ? 'Tool' : 'Workflow'
}

function formatStatusLabel(status: string): string {
  return status.charAt(0).toUpperCase() + status.slice(1)
}

function formatJsonValue(value?: Record<string, unknown>): string {
  return value ? JSON.stringify(value, null, 2) : '—'
}
</script>

<template>
  <section class="panel" role="region" aria-label="Pipeline node details">
    <section
      v-if="selectedViewNode.status === 'abandoned' && selectedViewNode.failureReason?.trim()"
      class="failure-reason"
      role="alert"
      aria-live="polite"
    >
      <strong>Failure reason</strong>
      <p>{{ selectedViewNode.failureReason }}</p>
    </section>

    <h2>{{ selectedViewNodeLabel || selectedViewNode.nodeId || 'Node Details' }}</h2>
    <section class="status-headline">
      <div class="status-headline__meta">
        <strong :class="`status-text--${selectedViewNode.status}`">{{ formatStatusLabel(selectedViewNode.status) }}</strong>
        <strong class="type-pill">{{ formatNodeType(selectedViewNode.type ?? 'workflow') }}</strong>
      </div>
      <p>{{ selectedNodeHeadline }}</p>
    </section>

    <div v-if="selectedNodeAgentSummary" class="summary-row">
      <span>Agent</span>
      <strong>{{ selectedNodeAgentSummary }}</strong>
    </div>

    <template v-if="selectedToolViewNode">
      <div v-if="showToolDirectExecutionExplanation" class="operator-callout">
        <strong>Tool node — executed directly, no interactive session</strong>
        <span>{{ selectedToolViewNode.toolProvenance?.toolName ?? selectedToolViewNode.toolName }}</span>
      </div>
      <div class="summary-row">
        <span>Tool</span>
        <strong>{{ selectedToolViewNode.toolProvenance?.toolName ?? selectedToolViewNode.toolName }}</strong>
      </div>
      <div class="read-block">
        <span>Resolved args</span>
        <pre>{{ formatJsonValue(selectedToolResolvedArgs ?? undefined) }}</pre>
      </div>
    </template>

    <details class="output-block" :open="selectedNodeOutput.length > 0">
      <summary>Output</summary>
      <pre v-if="selectedNodeOutput.length > 0">{{ selectedNodeOutput }}</pre>
      <pre v-else>Output will appear when this node completes</pre>
    </details>

    <div v-if="showExpiredSessionExplanation" class="operator-callout">
      <strong>Session ended — results preserved above</strong>
      <code class="mono">{{ selectedNodeReconnectSessionId }}</code>
    </div>

    <div v-if="selectedNodeCanReconnect && selectedNodeReconnectSessionId" class="action-row">
      <Button type="button" :disabled="openingAgents" @click="emit('open-agents')">
        {{ openingAgents ? 'Opening…' : 'Open in Agents tab' }}
      </Button>
    </div>

    <InspectorNodeActions :node="selectedViewNode" :node-id="selectedViewNode.nodeId" />

    <PipelineInspectorServerNodeTechnical
      v-if="shouldShowExpertDetails"
      :selected-view-node="selectedViewNode"
      :selected-workflow-view-node="selectedWorkflowViewNode"
      :selected-prompt-view-node="selectedPromptViewNode"
      :selected-thinking-view-node="selectedThinkingViewNode"
      :selected-tool-view-node="selectedToolViewNode"
      @copy="emit('copy', $event)"
    />
  </section>
</template>

<style scoped>
.panel { padding: 1rem; display: flex; flex-direction: column; gap: 0.875rem; background: var(--pipeline-inspector-surface); border-radius: 4px; }
.panel h2 { font-size: var(--text-xl); font-weight: var(--weight-title); }
.summary-row,.action-row,.read-block,.output-block { display: flex; flex-direction: column; gap: 0.375rem; color: var(--muted-foreground); font-size: var(--text-sm); }
.summary-row { flex-direction: row; align-items: center; justify-content: space-between; gap: 0.75rem; }
.summary-row span,.output-block summary,.read-block span { color: var(--muted-foreground); }
.status-headline {
  display: flex; flex-direction: column; gap: 0.5rem; padding: 0.875rem;
  border: 1px solid var(--pipeline-inspector-ghost-border); border-radius: 4px; background: var(--pipeline-inspector-well);
}
.status-headline__meta { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; }
.status-headline p { margin: 0; color: var(--foreground); }
.status-text--pending { color: var(--color-status-pending); }
.status-text--active { color: var(--color-status-active); }
.status-text--completed { color: var(--color-status-completed); }
.status-text--abandoned { color: var(--color-status-abandoned); }
.status-text--stuck { color: var(--color-status-stuck); }
.status-text--ready,
.status-text--paused { color: var(--color-status-ready); }
.status-text--cancelled { color: var(--color-status-abandoned); }
.type-pill {
  padding: 0.2rem 0.6rem; font-size: var(--text-sm); color: var(--foreground);
  border: 1px solid var(--pipeline-inspector-ghost-border); border-radius: 4px; background: var(--pipeline-inspector-well);
}
.read-block pre,.output-block {
  border: 1px solid var(--pipeline-inspector-ghost-border); border-radius: 4px; padding: 0.75rem; background: var(--pipeline-inspector-well);
}
.read-block pre,.output-block pre { margin: 0; white-space: pre-wrap; word-break: break-word; overflow-wrap: break-word; font: inherit; color: var(--foreground); }
.operator-callout {
  display: flex; flex-direction: column; gap: 0.35rem; padding: 0.75rem;
  border: 1px solid var(--pipeline-inspector-ghost-border); border-radius: 4px; background: var(--pipeline-inspector-well);
}
.failure-reason {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  padding: 0.875rem;
  border: 1px solid color-mix(in srgb, var(--color-status-abandoned) 55%, transparent);
  border-radius: 4px;
  background: linear-gradient(
    135deg,
    color-mix(in srgb, var(--color-status-abandoned) 10%, var(--pipeline-inspector-well)),
    color-mix(in srgb, var(--color-status-active) 10%, var(--pipeline-inspector-well))
  );
}
.failure-reason strong { color: var(--color-status-abandoned); }
.failure-reason p {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  overflow-wrap: break-word;
  color: var(--foreground);
}
.operator-callout strong { color: var(--foreground); }
.mono,.summary-row strong,.operator-callout span,.operator-callout code { min-width: 0; word-break: break-word; overflow-wrap: break-word; }
.mono { font-family: var(--font-code); }
</style>
