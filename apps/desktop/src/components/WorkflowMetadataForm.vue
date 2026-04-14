<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import type { WorkflowCategory } from '../lib/workflow-asset-types'
import { useWorkflowLibraryStore } from '../stores/workflow-library'

const selectFieldClass =
  'w-full bg-input text-foreground border border-border rounded-md px-3 py-2 text-caption outline-none focus:border-ring focus:ring-2 focus:ring-ring/20'

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
  <div v-if="workflow" class="workflow-form" role="form" aria-label="Workflow metadata">
    <div class="form-group">
      <label class="field-label">WORKFLOW ID</label>
      <code class="field-display">{{ workflow.id }}</code>
    </div>

    <div class="form-group">
      <label class="field-label" for="workflow-name">NAME</label>
      <Input id="workflow-name" v-model="workflowName" type="text" />
    </div>

    <div class="form-row">
      <div class="form-group">
        <label class="field-label" for="workflow-emoji">EMOJI</label>
        <Input id="workflow-emoji" v-model="workflowEmoji" type="text" />
      </div>

      <div class="form-group">
        <label class="field-label" for="workflow-category">CATEGORY</label>
        <select id="workflow-category" v-model="workflowCategory" :class="selectFieldClass">
          <option value="">Auto</option>
          <option v-for="option in CATEGORY_OPTIONS" :key="option" :value="option">
            {{ option }}
          </option>
        </select>
      </div>
    </div>

    <div class="form-group">
      <label class="field-label" for="workflow-description">DESCRIPTION</label>
      <Textarea
        id="workflow-description"
        v-model="workflowDescription"
        rows="4"
      />
    </div>

    <div class="form-group">
      <label class="field-label" for="workflow-triggers">TRIGGERS</label>
      <Input
        id="workflow-triggers"
        v-model="workflowTriggers"
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
.field-label { font: var(--weight-display) var(--text-tiny)/1.4 var(--font-code); color: var(--muted-foreground); text-transform: uppercase; letter-spacing: 0.05em; }
.field-display { font-family: var(--font-code); font-size: var(--text-caption); color: var(--muted-foreground); padding: var(--space-2) 0; line-height: 1.5; }
.toggle-card { display: flex; align-items: center; justify-content: space-between; gap: var(--space-3); padding: var(--space-3) var(--space-4); background: var(--card); border-radius: var(--radius-md); border: 1px solid var(--border); color: var(--foreground); cursor: pointer; text-align: left; }
.toggle-card--enabled { border-color: color-mix(in srgb, var(--color-node-workflow) 30%, var(--border)); box-shadow: 0 0 0 1px color-mix(in srgb, var(--color-node-workflow) 16%, transparent); }
.toggle-copy { font: var(--weight-body) var(--text-caption)/1.5 var(--font-body); }
.toggle-indicator { width: 36px; height: 20px; border-radius: 999px; background: var(--input); display: flex; align-items: center; padding: 2px; transition: background var(--transition-fast); }
.toggle-indicator--enabled { background: var(--color-node-workflow); justify-content: flex-end; }
.indicator-dot { width: 16px; height: 16px; border-radius: 50%; background: var(--foreground); }
</style>
