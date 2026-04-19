<script setup lang="ts">
import type { ToolProvenance, WorkflowStepEntry } from '../lib/pipeline-types';
import ThinkingTraceView from './ThinkingTraceView.vue';
import PromptAssemblyPreview from './PromptAssemblyPreview.vue';
import WorkflowStepProgress from './WorkflowStepProgress.vue';

interface ThinkingTraceEntry {
  thought_number: number;
  status: string;
  text: string;
}

const props = defineProps<{
  assembledPrompt: string | null;
  nodeOutput: string | null;
  thinkingTrace: ThinkingTraceEntry[] | null;
  toolProvenance: ToolProvenance | null;
  workflowSteps: WorkflowStepEntry[] | null;
}>();

function formatJson(value: Record<string, unknown> | null | undefined): string {
  return value ? JSON.stringify(value, null, 2) : '—';
}
</script>

<template>
  <div class="zoom-content">
    <PromptAssemblyPreview v-if="props.assembledPrompt" :assembled-prompt="props.assembledPrompt" />

    <ThinkingTraceView v-if="props.thinkingTrace && props.thinkingTrace.length > 0" :trace="props.thinkingTrace" />

    <WorkflowStepProgress v-if="props.workflowSteps && props.workflowSteps.length > 0" :steps="props.workflowSteps" />

    <details v-if="props.nodeOutput" class="zoom-section" open>
      <summary class="zoom-section-title">Output</summary>
      <pre class="zoom-pre">{{ props.nodeOutput }}</pre>
    </details>

    <details v-if="props.toolProvenance" class="zoom-section" open>
      <summary class="zoom-section-title">Tool Provenance</summary>
      <div class="zoom-metadata">
        <span class="item-label">Tool</span>
        <strong>{{ props.toolProvenance.toolName }}</strong>
      </div>
      <pre class="zoom-pre">{{ formatJson(props.toolProvenance.resolvedArgs ?? null) }}</pre>
      <pre v-if="props.toolProvenance.responseSummary" class="zoom-pre">{{ props.toolProvenance.responseSummary }}</pre>
    </details>

    <div
      v-if="!props.assembledPrompt && !props.thinkingTrace && !props.workflowSteps && !props.nodeOutput && !props.toolProvenance"
      class="zoom-empty"
    >
      This node hasn't produced any output yet.
    </div>
  </div>
</template>

<style scoped>
.zoom-content {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.zoom-section-title {
  font-family: var(--font-display);
  font-size: var(--text-caption);
  font-weight: var(--weight-title);
  color: var(--color-text-primary);
  cursor: pointer;
  padding: var(--space-2) 0;
  list-style: none;
}

.zoom-section-title::before {
  content: '▸ ';
}

.zoom-section[open] > .zoom-section-title::before {
  content: '▾ ';
}

.zoom-pre {
  margin: var(--space-2) 0 0;
  padding: var(--space-3);
  font-family: var(--font-code);
  font-size: var(--text-tiny);
  line-height: 1.5;
  color: var(--color-text-secondary);
  background: var(--color-bg-app);
  border-radius: var(--radius-md);
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 300px;
  overflow-y: auto;
}

.zoom-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.zoom-list-item {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  padding: var(--space-2);
  background: var(--color-bg-surface);
  border-radius: var(--radius-md);
}

.zoom-metadata {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  margin-top: var(--space-2);
}

.item-label {
  font-family: var(--font-code);
  font-size: var(--text-tiny);
  font-weight: var(--weight-title);
  color: var(--color-text-primary);
}

.item-status {
  font-size: var(--text-tiny);
  color: var(--color-text-secondary);
}

.zoom-status--active { color: var(--color-status-active); }
.zoom-status--completed { color: var(--color-status-completed); }
.zoom-status--abandoned { color: var(--color-status-abandoned); }
.zoom-status--ready { color: var(--color-status-ready); }

.zoom-empty {
  padding: var(--space-8);
  text-align: center;
  color: var(--color-text-secondary);
  font-size: var(--text-caption);
}
</style>
