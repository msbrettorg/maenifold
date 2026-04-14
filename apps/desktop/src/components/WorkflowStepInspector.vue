<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useWorkflowLibraryStore } from '../stores/workflow-library'
import type { ReasoningEffort, WorkflowStepDefinition, ToolHintPriority } from '../lib/workflow-asset-types'
import WorkflowMetadataForm from './WorkflowMetadataForm.vue'

const props = defineProps<{
  selectedStepId: string | null
}>()

const store = useWorkflowLibraryStore()
const { currentDefinition, draftDefinition } = storeToRefs(store)

const displayedDefinition = computed(() => draftDefinition.value ?? currentDefinition.value)
const selectedStep = computed<WorkflowStepDefinition | null>(() => {
  if (!props.selectedStepId || !displayedDefinition.value) return null
  return (
    displayedDefinition.value.steps.find(
      (step: { id: string }) => step.id === props.selectedStepId,
    ) ?? null
  )
})
const reasoningLevels: ReasoningEffort[] = ['low', 'medium', 'high']

const stepName = computed({
  get: () => selectedStep.value?.name ?? '',
  set: (value: string) => {
    if (selectedStep.value) store.updateStepName(selectedStep.value.id, value)
  },
})

const stepDescription = computed({
  get: () => selectedStep.value?.description ?? '',
  set: (value: string) => {
    if (selectedStep.value) store.updateStepDescription(selectedStep.value.id, value)
  },
})

const selectedEffort = computed<ReasoningEffort>({
  get: () => selectedStep.value?.metadata?.reasoning_effort ?? 'medium',
  set: (value) => {
    if (selectedStep.value) store.setStepReasoningEffort(selectedStep.value.id, value)
  },
})

const toolHintEntries = computed(() => {
  const hints = selectedStep.value?.metadata?.toolHints
  if (!hints) return []
  return Object.entries(hints).map(([name, hint]) => ({ name, ...hint }))
})

function priorityClass(priority: ToolHintPriority): string {
  switch (priority) {
    case 'mandatory':
      return 'priority-mandatory'
    case 'recommended':
      return 'priority-recommended'
    case 'optional':
      return 'priority-optional'
    default:
      return ''
  }
}
</script>

<template>
  <aside class="step-inspector" aria-label="Step inspector">
    <div v-if="!displayedDefinition" class="empty-state">
      <span class="empty-text">No workflow selected</span>
    </div>

    <template v-else-if="selectedStep">
      <h2 class="panel-header">Step Properties</h2>
      <div class="step-id-row">
        <span class="step-id-label">ID:</span>
        <code class="step-id-value">{{ selectedStep.id }}</code>
      </div>

      <div class="form-group">
        <label class="field-label">STEP NAME</label>
        <input v-model="stepName" class="field-input" type="text" />
      </div>

      <div class="form-group">
        <label class="field-label">DESCRIPTION</label>
        <textarea v-model="stepDescription" class="field-textarea" rows="4" />
      </div>

      <div class="form-group">
        <label class="field-label">REASONING EFFORT</label>
        <div class="effort-segments" role="group" aria-label="Reasoning effort">
          <button
            v-for="level in reasoningLevels"
            :key="level"
            :class="['effort-segment', { active: selectedEffort === level }]"
            type="button"
            @click="selectedEffort = level"
          >
            {{ level.toUpperCase() }}
          </button>
        </div>
      </div>

      <div class="form-group">
        <button
          class="thinking-card"
          :class="{ enabled: selectedStep.requiresSequentialThinking }"
          type="button"
          @click="store.setStepRequiresSequentialThinking(selectedStep.id, !selectedStep.requiresSequentialThinking)"
        >
          <div class="thinking-text">
            <span class="thinking-title">Sequential Thinking</span>
            <span class="thinking-subtitle">ENABLE CHAIN-OF-THOUGHT</span>
          </div>
          <span class="thinking-indicator" :class="{ on: selectedStep.requiresSequentialThinking }">
            <span class="indicator-dot" />
          </span>
        </button>
      </div>

      <div v-if="toolHintEntries.length > 0" class="form-group">
        <label class="field-label">TOOL HINTS</label>
        <div class="tool-chips">
          <span
            v-for="entry in toolHintEntries"
            :key="entry.name"
            class="tool-chip"
            :title="entry.reason"
          >
            {{ entry.name }}
            <span class="priority-badge" :class="priorityClass(entry.priority)">
              {{ entry.priority }}
            </span>
          </span>
        </div>
      </div>
    </template>

    <template v-else>
      <h2 class="panel-header">Workflow</h2>
      <div class="workflow-meta">
        <WorkflowMetadataForm />
      </div>
    </template>
  </aside>
</template>

<style scoped>
.step-inspector { height: 100%; background: var(--bg-panel); overflow-y: auto; display: flex; flex-direction: column; padding-bottom: var(--space-4); }
.panel-header { min-height: 44px; margin: 0 0 var(--space-4); padding: 0 var(--space-4); border-bottom: 1px solid var(--border-default); display: flex; align-items: center; gap: var(--space-2); font-size: 0; color: transparent; flex-shrink: 0; }
.panel-header::before { content: ''; width: 6px; height: 6px; border-radius: 50%; background: var(--color-node-workflow); }
.panel-header::after { content: 'STEP INSPECTOR'; font: var(--weight-title) var(--text-tiny)/1 var(--font-code); letter-spacing: 0.06em; text-transform: uppercase; color: var(--text-secondary); }
.step-inspector:has(.workflow-meta) .panel-header::after { content: 'WORKFLOW'; }
.step-id-row, .workflow-meta, .form-group { margin: 0 var(--space-4); }
.step-id-row { display: flex; align-items: center; gap: var(--space-2); margin-bottom: var(--space-5); padding: var(--space-2) var(--space-3); border: 1px solid var(--border-default); border-radius: var(--radius-md); background: var(--surface-container); flex-wrap: wrap; }
.workflow-meta { display: flex; flex-direction: column; gap: var(--space-5); }
.form-group { display: flex; flex-direction: column; gap: var(--space-1); }
.form-group + .form-group { margin-top: var(--space-5); }
.step-id-label, .field-label { font: var(--weight-display) var(--text-tiny)/1.4 var(--font-code); color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em; }
.step-id-value, .meta-mono { font-family: var(--font-code); font-size: var(--text-tiny); color: var(--text-secondary); }
.field-input, .field-textarea { width: 100%; background: var(--bg-input); color: var(--text-primary); border: 1px solid var(--border-default); border-radius: var(--radius-md); padding: var(--space-2) var(--space-3); font: var(--weight-body) var(--text-caption)/1.5 var(--font-body); outline: none; transition: border-color var(--transition-fast), box-shadow var(--transition-fast); }
.field-textarea { min-height: 80px; resize: vertical; }
.field-input:hover:not(:focus), .field-textarea:hover:not(:focus) { border-color: color-mix(in srgb, var(--border-default) 150%, transparent); }
.field-input:focus, .field-textarea:focus { border-color: var(--border-focus); box-shadow: 0 0 0 2px color-mix(in srgb, var(--border-focus) 20%, transparent); }
.effort-segments { display: grid; grid-template-columns: repeat(3, 1fr); border-radius: var(--radius-md); overflow: hidden; border: 1px solid var(--border-default); }
.effort-segment { text-align: center; padding: var(--space-2) var(--space-1); background: var(--bg-input); color: var(--text-muted); border: 0; font: var(--weight-title) var(--text-tiny)/1.4 var(--font-code); letter-spacing: 0.05em; text-transform: uppercase; cursor: pointer; }
.effort-segment + .effort-segment { border-left: 1px solid var(--border-default); }
.effort-segment.active { background: color-mix(in srgb, var(--color-node-workflow) 16%, var(--bg-input)); color: var(--color-node-workflow); }
.thinking-card { display: flex; align-items: center; justify-content: space-between; gap: var(--space-3); padding: var(--space-3) var(--space-4); background: var(--surface-container); border-radius: var(--radius-md); border: 1px solid var(--border-default); cursor: pointer; }
.thinking-card.enabled { border-color: color-mix(in srgb, var(--color-node-workflow) 30%, var(--border-default)); box-shadow: 0 0 0 1px color-mix(in srgb, var(--color-node-workflow) 16%, transparent); }
.thinking-text { display: flex; flex-direction: column; gap: 2px; text-align: left; }
.thinking-title, .meta-value { font: var(--weight-body) var(--text-caption)/1.5 var(--font-body); color: var(--text-primary); }
.thinking-subtitle { font: var(--weight-display) var(--text-tiny)/1.4 var(--font-code); color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em; }
.thinking-indicator { width: 36px; height: 20px; border-radius: 999px; background: var(--bg-input); display: flex; align-items: center; padding: 2px; transition: background var(--transition-fast); }
.thinking-indicator.on { background: var(--color-node-workflow); justify-content: flex-end; }
.indicator-dot { width: 16px; height: 16px; border-radius: 50%; background: var(--text-primary); }
.tool-chips { display: flex; flex-wrap: wrap; gap: var(--space-2); }
.tool-chip { display: inline-flex; align-items: center; gap: var(--space-1); padding: var(--space-1) var(--space-2); border: 1px solid var(--border-default); border-radius: var(--radius-sm); background: var(--surface-container); color: var(--text-primary); font: var(--weight-code) var(--text-tiny)/1.4 var(--font-code); }
.priority-badge { padding: 1px 4px; border-radius: 3px; font: var(--weight-title) var(--text-2xs)/1.4 var(--font-code); letter-spacing: 0.03em; text-transform: uppercase; }
.priority-mandatory { background: color-mix(in srgb, var(--color-status-abandoned) 25%, transparent); color: var(--color-status-abandoned); }
.priority-recommended { background: color-mix(in srgb, var(--color-node-workflow) 20%, transparent); color: var(--color-node-workflow); }
.priority-optional { background: color-mix(in srgb, var(--text-muted) 20%, transparent); color: var(--text-muted); }
.meta-emoji { margin-right: var(--space-1); }
.empty-state { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: var(--space-3); height: 100%; padding: var(--space-4); }
.empty-text { color: var(--text-muted); font-size: var(--text-caption); text-align: center; }
</style>
