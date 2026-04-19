<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
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

function formatReasoningLevel(level: ReasoningEffort): string {
  return level.charAt(0).toUpperCase() + level.slice(1)
}

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
    <div v-if="!displayedDefinition" class="inspector-empty" aria-hidden="true" />

    <template v-else-if="selectedStep">
      <h2 class="panel-header">Step properties</h2>

      <section class="inspector-section" aria-labelledby="inspector-section-prompt">
        <h3 id="inspector-section-prompt" class="section-label">Prompt</h3>
        <div class="form-group">
          <label class="field-label">Step name</label>
          <Input v-model="stepName" type="text" />
        </div>
        <div class="form-group">
          <label class="field-label">Description</label>
          <Textarea v-model="stepDescription" rows="4" />
        </div>
      </section>

      <section class="inspector-section" aria-labelledby="inspector-section-tools">
        <h3 id="inspector-section-tools" class="section-label">Tools &amp; permissions</h3>
        <div v-if="toolHintEntries.length > 0" class="form-group">
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
        <p v-else class="section-empty">No tool hints declared for this step.</p>
      </section>

      <section class="inspector-section" aria-labelledby="inspector-section-thinking">
        <h3 id="inspector-section-thinking" class="section-label">Sequential thinking</h3>
        <div class="form-group">
          <button
            class="thinking-card"
            :class="{ enabled: selectedStep.requiresSequentialThinking }"
            type="button"
            @click="store.setStepRequiresSequentialThinking(selectedStep.id, !selectedStep.requiresSequentialThinking)"
          >
            <div class="thinking-text">
              <span class="thinking-title">Sequential thinking</span>
              <span class="thinking-subtitle">Enable chain-of-thought</span>
            </div>
            <span class="thinking-indicator" :class="{ on: selectedStep.requiresSequentialThinking }">
              <span class="indicator-dot" />
            </span>
          </button>
        </div>
        <div class="form-group">
          <label class="field-label">Reasoning effort</label>
          <div class="effort-segments" role="group" aria-label="Reasoning effort">
            <button
              v-for="level in reasoningLevels"
              :key="level"
              :class="['effort-segment', { active: selectedEffort === level }]"
              type="button"
              @click="selectedEffort = level"
            >
              {{ formatReasoningLevel(level) }}
            </button>
          </div>
        </div>
      </section>

      <section class="inspector-section inspector-section--last" aria-labelledby="inspector-section-meta">
        <h3 id="inspector-section-meta" class="section-label">Meta</h3>
        <div class="form-group">
          <label class="field-label">Step ID</label>
          <code class="step-id-value">{{ selectedStep.id }}</code>
        </div>
      </section>
    </template>

    <template v-else>
      <h2 class="panel-header">Workflow</h2>
      <div class="workflow-meta">
        <WorkflowMetadataForm />
      </div>
      <div class="inspector-empty-state" role="status">
        <span class="empty-title">Select a step</span>
        <span class="empty-text">Pick a step from the timeline to edit its prompt, tools, and reasoning settings.</span>
      </div>
    </template>
  </aside>
</template>

<style scoped>
.step-inspector { height: 100%; background: var(--color-bg-elevated); overflow-y: auto; display: flex; flex-direction: column; padding-bottom: var(--space-4); }
.panel-header { min-height: 44px; margin: 0 0 var(--space-4); padding: 0 var(--space-4); background: color-mix(in srgb, var(--color-bg-elevated) 30%, var(--color-bg-app)); display: flex; align-items: center; gap: var(--space-2); font-size: 0; color: transparent; flex-shrink: 0; }
.panel-header::before { content: ''; width: 6px; height: 6px; border-radius: var(--radius); background: var(--color-node-workflow); }
.panel-header::after { content: 'Step inspector'; font: var(--weight-title) var(--text-tiny)/1 var(--font-code); letter-spacing: 0.06em; color: var(--color-text-secondary); }
.step-inspector:has(.workflow-meta) .panel-header::after { content: 'Workflow'; }
.step-id-row, .workflow-meta, .form-group { margin: 0 var(--space-4); }
.step-id-row { display: flex; align-items: center; gap: var(--space-2); margin-bottom: var(--space-5); padding: var(--space-2) var(--space-3); border-radius: var(--radius-md); background: color-mix(in srgb, var(--color-bg-elevated) 82%, var(--color-bg-app)); box-shadow: inset 0 0 0 0.75px color-mix(in srgb, var(--color-border-subtle) 65%, transparent); flex-wrap: wrap; }
.workflow-meta { display: flex; flex-direction: column; gap: var(--space-5); }
.form-group { display: flex; flex-direction: column; gap: var(--space-1); }
.form-group + .form-group { margin-top: var(--space-5); }
.step-id-label, .field-label { font: var(--weight-display) var(--text-tiny)/1.4 var(--font-code); color: var(--color-text-secondary); letter-spacing: 0.05em; }
.step-id-value, .meta-mono { font-family: var(--font-code); font-size: var(--text-tiny); color: var(--color-text-secondary); }
.effort-segments { display: grid; grid-template-columns: repeat(3, 1fr); border-radius: var(--radius-md); overflow: hidden; background: color-mix(in srgb, var(--color-border-subtle) 75%, var(--color-bg-elevated)); box-shadow: inset 0 0 0 0.75px color-mix(in srgb, var(--color-border-subtle) 65%, transparent); }
.effort-segment { text-align: center; padding: var(--space-2) var(--space-1); background: var(--color-border-subtle); color: var(--color-text-secondary); border: 0; font: var(--weight-title) var(--text-tiny)/1.4 var(--font-code); letter-spacing: 0.05em; cursor: pointer; }
.effort-segment + .effort-segment { box-shadow: inset 0.75px 0 0 color-mix(in srgb, var(--color-border-subtle) 75%, transparent); }
.effort-segment.active { background: color-mix(in srgb, var(--color-node-workflow) 16%, var(--color-border-subtle)); color: var(--color-node-workflow); }
.thinking-card { display: flex; align-items: center; justify-content: space-between; gap: var(--space-3); padding: var(--space-3) var(--space-4); background: color-mix(in srgb, var(--color-bg-elevated) 86%, var(--color-bg-app)); border-radius: var(--radius-md); box-shadow: inset 0 0 0 0.75px color-mix(in srgb, var(--color-border-subtle) 65%, transparent); cursor: pointer; transition: background-color var(--transition-fast), box-shadow var(--transition-fast); }
.thinking-card.enabled { background: color-mix(in srgb, var(--color-node-workflow) 10%, var(--color-bg-elevated)); box-shadow: inset 0 0 0 0.75px color-mix(in srgb, var(--color-node-workflow) 45%, var(--color-border-subtle)), 0 0 0 2px color-mix(in srgb, var(--color-node-workflow) 12%, transparent); }
.thinking-text { display: flex; flex-direction: column; gap: 2px; text-align: left; }
.thinking-title, .meta-value { font: var(--weight-body) var(--text-caption)/1.5 var(--font-body); color: var(--color-text-primary); }
.thinking-subtitle { font: var(--weight-display) var(--text-tiny)/1.4 var(--font-code); color: var(--color-text-secondary); letter-spacing: 0.05em; }
.thinking-indicator { width: 36px; height: 20px; border-radius: 4px; background: var(--color-border-subtle); display: flex; align-items: center; padding: 2px; transition: background var(--transition-fast); }
.thinking-indicator.on { background: var(--color-node-workflow); justify-content: flex-end; }
.indicator-dot { width: 16px; height: 16px; border-radius: var(--radius); background: var(--color-text-primary); }
.tool-chips { display: flex; flex-wrap: wrap; gap: var(--space-2); }
.tool-chip { display: inline-flex; align-items: center; gap: var(--space-1); padding: var(--space-1) var(--space-2); border-radius: var(--radius-sm); background: color-mix(in srgb, var(--color-bg-elevated) 80%, var(--color-bg-app)); box-shadow: inset 0 0 0 0.75px color-mix(in srgb, var(--color-border-subtle) 75%, transparent); color: var(--color-text-primary); font: var(--weight-code) var(--text-tiny)/1.4 var(--font-code); }
.priority-badge { padding: 2px 4px; border-radius: 3px; font: var(--weight-title) var(--text-2xs)/1.4 var(--font-code); letter-spacing: 0.03em; }
.priority-mandatory { background: color-mix(in srgb, var(--color-status-abandoned) 25%, transparent); color: var(--color-status-abandoned); }
.priority-recommended { background: color-mix(in srgb, var(--color-node-workflow) 20%, transparent); color: var(--color-node-workflow); }
.priority-optional { background: color-mix(in srgb, var(--color-text-secondary) 20%, transparent); color: var(--color-text-secondary); }
.meta-emoji { margin-right: var(--space-1); }
.inspector-empty { flex: 1; }
</style>
