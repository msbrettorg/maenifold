<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import type { WorkflowCategory } from '../lib/workflow-asset-types'
import { useWorkflowLibraryStore } from '../stores/workflow-library'

const CATEGORY_OPTIONS: WorkflowCategory[] = [
  'reasoning',
  'scientific',
  'multi-agent',
  'devops',
  'memory',
  'custom',
]

const store = useWorkflowLibraryStore()
const { currentDefinition, draftDefinition } = storeToRefs(store)

const workflow = computed(() => draftDefinition.value ?? currentDefinition.value)
const workflowName = computed({
  get: () => workflow.value?.name ?? '',
  set: (value: string) => store.updateWorkflowName(value),
})
const workflowEmoji = computed({
  get: () => workflow.value?.emoji ?? '',
  set: (value: string) => store.updateWorkflowEmoji(value),
})
const workflowDescription = computed({
  get: () => workflow.value?.description ?? '',
  set: (value: string) => store.updateWorkflowDescription(value),
})
const workflowTriggers = computed({
  get: () => (workflow.value?.triggers ?? []).join(', '),
  set: (value: string) =>
    store.updateWorkflowTriggers(
      value
        .split(',')
        .map((entry) => entry.trim())
        .filter(Boolean),
    ),
})
const workflowCategory = computed({
  get: () => workflow.value?.category ?? '',
  set: (value: string) =>
    store.updateWorkflowCategory(value ? (value as WorkflowCategory) : null),
})
</script>

<template>
  <div v-if="workflow" class="workflow-form">
    <div class="form-group">
      <label class="field-label">WORKFLOW ID</label>
      <code class="field-display">{{ workflow.id }}</code>
    </div>

    <div class="form-group">
      <label class="field-label" for="workflow-name">NAME</label>
      <input id="workflow-name" v-model="workflowName" class="field-input" type="text" />
    </div>

    <div class="form-row">
      <div class="form-group">
        <label class="field-label" for="workflow-emoji">EMOJI</label>
        <input id="workflow-emoji" v-model="workflowEmoji" class="field-input" type="text" />
      </div>

      <div class="form-group">
        <label class="field-label" for="workflow-category">CATEGORY</label>
        <select id="workflow-category" v-model="workflowCategory" class="field-input">
          <option value="">Auto</option>
          <option v-for="option in CATEGORY_OPTIONS" :key="option" :value="option">
            {{ option }}
          </option>
        </select>
      </div>
    </div>

    <div class="form-group">
      <label class="field-label" for="workflow-description">DESCRIPTION</label>
      <textarea
        id="workflow-description"
        v-model="workflowDescription"
        class="field-textarea"
        rows="4"
      />
    </div>

    <div class="form-group">
      <label class="field-label" for="workflow-triggers">TRIGGERS</label>
      <input
        id="workflow-triggers"
        v-model="workflowTriggers"
        class="field-input"
        placeholder="research, plan, investigate"
        type="text"
      />
    </div>

    <div class="form-group">
      <label class="field-label">SEQUENTIAL THINKING</label>
      <button
        class="toggle-card"
        :class="{ 'toggle-card--enabled': workflow.sequentialThinkingEnabled }"
        type="button"
        @click="store.updateWorkflowSequentialThinkingEnabled(!workflow.sequentialThinkingEnabled)"
      >
        <span class="toggle-copy">Enable workflow-level sequential thinking</span>
        <span class="toggle-indicator" :class="{ 'toggle-indicator--enabled': workflow.sequentialThinkingEnabled }">
          <span class="indicator-dot" />
        </span>
      </button>
    </div>

    <div class="form-group">
      <label class="field-label">MULTI-AGENT</label>
      <button
        class="toggle-card"
        :class="{ 'toggle-card--enabled': workflow.multiAgent }"
        type="button"
        @click="store.updateWorkflowMultiAgent(!workflow.multiAgent)"
      >
        <span class="toggle-copy">Mark this workflow as multi-agent</span>
        <span class="toggle-indicator" :class="{ 'toggle-indicator--enabled': workflow.multiAgent }">
          <span class="indicator-dot" />
        </span>
      </button>
    </div>
  </div>
</template>

<style scoped>
.workflow-form { display: flex; flex-direction: column; gap: var(--space-5); }
.form-row { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: var(--space-3); }
.form-group { display: flex; flex-direction: column; gap: var(--space-1); }
.field-label { font: 500 var(--text-xs)/1.4 var(--font-mono); color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em; }
.field-display { font-family: var(--font-mono); font-size: var(--text-sm); color: var(--text-secondary); padding: var(--space-2) 0; line-height: 1.5; }
.field-input, .field-textarea { width: 100%; background: var(--bg-input); color: var(--text-primary); border: 1px solid var(--border-default); border-radius: var(--radius-md); padding: var(--space-2) var(--space-3); font: 400 var(--text-sm)/1.5 var(--font-sans); outline: none; transition: border-color var(--transition-fast), box-shadow var(--transition-fast); }
.field-input:hover:not(:focus), .field-textarea:hover:not(:focus) { border-color: color-mix(in srgb, var(--border-default) 150%, transparent); }
.field-input:focus, .field-textarea:focus { border-color: var(--border-focus); box-shadow: 0 0 0 2px color-mix(in srgb, var(--border-focus) 20%, transparent); }
.field-textarea { min-height: 80px; resize: vertical; }
.toggle-card { display: flex; align-items: center; justify-content: space-between; gap: var(--space-3); padding: var(--space-3) var(--space-4); background: var(--surface-container); border-radius: var(--radius-md); border: 1px solid var(--border-default); color: var(--text-primary); cursor: pointer; text-align: left; }
.toggle-card--enabled { border-color: color-mix(in srgb, var(--color-node-workflow) 30%, var(--border-default)); box-shadow: 0 0 0 1px color-mix(in srgb, var(--color-node-workflow) 16%, transparent); }
.toggle-copy { font: 400 var(--text-sm)/1.5 var(--font-sans); }
.toggle-indicator { width: 36px; height: 20px; border-radius: 999px; background: var(--bg-input); display: flex; align-items: center; padding: 2px; transition: background var(--transition-fast); }
.toggle-indicator--enabled { background: var(--color-node-workflow); justify-content: flex-end; }
.indicator-dot { width: 16px; height: 16px; border-radius: 50%; background: var(--text-primary); }
</style>
