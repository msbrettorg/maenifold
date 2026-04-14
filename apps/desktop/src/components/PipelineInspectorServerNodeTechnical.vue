<script setup lang="ts">
import { Button } from '@/components/ui/button'
import PromptAssemblyPreview from './PromptAssemblyPreview.vue'
import ThinkingTraceView from './ThinkingTraceView.vue'
import WorkflowStepProgress from './WorkflowStepProgress.vue'
import type {
  PromptViewNode,
  SequentialThinkingViewNode,
  ToolViewNode,
  ViewNode,
  WorkflowViewNode,
} from '../lib/pipeline-types'

defineProps<{
  selectedViewNode: ViewNode
  selectedWorkflowViewNode: WorkflowViewNode | null
  selectedPromptViewNode: PromptViewNode | null
  selectedThinkingViewNode: SequentialThinkingViewNode | null
  selectedToolViewNode: ToolViewNode | null
}>()

const emit = defineEmits<{
  (e: 'copy', value: string): void
}>()

function displayValue(value?: string | number | null): string {
  return value === undefined || value === null || value === '' ? '—' : String(value)
}

function formatJsonValue(value?: Record<string, unknown>): string {
  return value ? JSON.stringify(value, null, 2) : '—'
}

function hasAssembledPrompt(value?: string | null): value is string {
  return value !== undefined && value !== null
}

function hasResponseSummary(value?: string | null): value is string {
  return value !== undefined && value !== null && value.trim().length > 0
}
</script>

<template>
  <details class="expert-disclosure" role="region" aria-label="Technical details">
    <summary>Technical Details</summary>

    <div class="expert-content">
      <div class="summary-row">
        <span>Session ID</span>
        <div class="copy-row">
          <strong class="mono">{{ selectedViewNode.sessionId }}</strong>
          <Button class="copy-button" variant="ghost" type="button" @click="emit('copy', selectedViewNode.sessionId)">Copy</Button>
        </div>
      </div>

      <div v-if="selectedViewNode.acpSessionId" class="summary-row">
        <span>ACP session</span>
        <div class="copy-row">
          <strong class="mono">{{ selectedViewNode.acpSessionId }}</strong>
          <Button class="copy-button" variant="ghost" type="button" @click="emit('copy', selectedViewNode.acpSessionId ?? '')">Copy</Button>
        </div>
      </div>

      <div v-if="selectedViewNode.thinkingSessionId" class="summary-row">
        <span>Thinking session</span>
        <div class="copy-row">
          <strong class="mono">{{ selectedViewNode.thinkingSessionId }}</strong>
          <Button class="copy-button" variant="ghost" type="button" @click="emit('copy', selectedViewNode.thinkingSessionId ?? '')">Copy</Button>
        </div>
      </div>

      <div v-if="selectedViewNode.attempts > 1" class="summary-row">
        <span>Attempts</span>
        <strong>{{ selectedViewNode.attempts }}</strong>
      </div>

      <template v-if="selectedWorkflowViewNode">
        <div class="summary-row"><span>Workflow ID</span><strong>{{ selectedWorkflowViewNode.workflowId }}</strong></div>
        <div class="summary-row"><span>ACP client</span><strong>{{ displayValue(selectedWorkflowViewNode.acpClient) }}</strong></div>
        <div v-if="selectedWorkflowViewNode.prompt" class="read-block"><span>Prompt template</span><pre>{{ displayValue(selectedWorkflowViewNode.prompt) }}</pre></div>
        <PromptAssemblyPreview v-if="hasAssembledPrompt(selectedWorkflowViewNode.assembled_prompt)" :assembled-prompt="selectedWorkflowViewNode.assembled_prompt" />
        <WorkflowStepProgress v-if="selectedWorkflowViewNode.workflow_steps != null" :steps="selectedWorkflowViewNode.workflow_steps" />
      </template>

      <template v-else-if="selectedPromptViewNode">
        <div class="read-block"><span>Prompt template</span><pre>{{ selectedPromptViewNode.prompt }}</pre></div>
        <PromptAssemblyPreview v-if="hasAssembledPrompt(selectedPromptViewNode.assembled_prompt)" :assembled-prompt="selectedPromptViewNode.assembled_prompt" />
      </template>

      <template v-else-if="selectedThinkingViewNode">
        <div class="read-block"><span>Topic</span><pre>{{ selectedThinkingViewNode.topic }}</pre></div>
        <div class="summary-row"><span>Total thoughts</span><strong>{{ displayValue(selectedThinkingViewNode.totalThoughts) }}</strong></div>
        <div class="summary-row"><span>Analysis type</span><strong>{{ displayValue(selectedThinkingViewNode.analysisType) }}</strong></div>
        <PromptAssemblyPreview v-if="hasAssembledPrompt(selectedThinkingViewNode.assembled_prompt)" :assembled-prompt="selectedThinkingViewNode.assembled_prompt" />
        <ThinkingTraceView v-if="selectedThinkingViewNode.thinking_trace != null" :trace="selectedThinkingViewNode.thinking_trace" />
      </template>

      <template v-else-if="selectedToolViewNode">
        <div class="summary-row"><span>Tool name</span><strong>{{ selectedToolViewNode.toolName }}</strong></div>
        <div class="read-block"><span>Tool args</span><pre>{{ formatJsonValue(selectedToolViewNode.toolArgs) }}</pre></div>
        <div v-if="hasResponseSummary(selectedToolViewNode.toolProvenance?.responseSummary)" class="read-block">
          <span>Response summary</span>
          <pre>{{ selectedToolViewNode.toolProvenance?.responseSummary }}</pre>
        </div>
      </template>
    </div>
  </details>
</template>

<style scoped>
.expert-disclosure,.read-block { display: flex; flex-direction: column; gap: 0.375rem; color: var(--muted-foreground); font-size: var(--text-sm); }
.expert-disclosure {
  padding: 0.875rem; border: 1px solid var(--pipeline-inspector-ghost-border);
  border-radius: 4px; background: var(--pipeline-inspector-well);
}
.expert-content { display: flex; flex-direction: column; gap: 0.875rem; margin-top: 0.75rem; }
.summary-row { display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; }
.summary-row span,.expert-disclosure summary,.read-block span { color: var(--muted-foreground); }
.read-block pre {
  margin: 0; border: 1px solid var(--pipeline-inspector-ghost-border); border-radius: 4px; padding: 0.75rem;
  background: var(--pipeline-inspector-surface); white-space: pre-wrap; word-break: break-word; overflow-wrap: break-word; font: inherit; color: var(--foreground);
}
.copy-row { display: flex; align-items: center; gap: 0.5rem; justify-content: flex-end; flex-wrap: wrap; }
.copy-button { padding: 0.25rem 0.5rem; }
.mono,.copy-row strong { min-width: 0; word-break: break-word; overflow-wrap: break-word; font-family: var(--font-code); }
</style>
