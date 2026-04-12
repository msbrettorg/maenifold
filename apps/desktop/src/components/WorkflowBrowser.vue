<script setup lang="ts">
import { storeToRefs } from 'pinia'
import { computed, ref } from 'vue'
import type { WorkflowCategory } from '../lib/workflow-asset-types'
import { usePipelineMcp } from '../composables/usePipelineMcp'
import { usePipelineStore } from '../stores/pipeline'
import { useWorkflowLibraryStore } from '../stores/workflow-library'

const pipelineStore = usePipelineStore()
const workflowMcp = usePipelineMcp()
const store = useWorkflowLibraryStore()
const { currentId, categoryFilter, error, filteredWorkflows, isDirty, searchQuery, summaries } = storeToRefs(store)
const browserError = computed(() => error.value ?? pipelineStore.error ?? null)

interface CategoryChip {
  label: string
  value: WorkflowCategory | 'all'
}

const chips: CategoryChip[] = [
  { label: 'All', value: 'all' },
  { label: 'Reasoning', value: 'reasoning' },
  { label: 'Scientific', value: 'scientific' },
  { label: 'Multi-Agent', value: 'multi-agent' },
  { label: 'DevOps', value: 'devops' },
  { label: 'Memory', value: 'memory' },
]

const importError = ref<string | null>(null)
const importConflict = ref<{ id: string, json: string, name: string } | null>(null)
const importInputRef = ref<HTMLInputElement | null>(null)

function clearImportState() {
  importError.value = null
  importConflict.value = null
}

function nextCopyId(id: string) {
  let suffix = 1
  let candidate = `${id}-copy`
  while (summaries.value.some((workflow) => workflow.id === candidate)) {
    suffix += 1
    candidate = `${id}-copy-${suffix}`
  }
  return candidate
}

function prepareImportedWorkflow(json: string, resolution?: 'overwrite' | 'copy') {
  let parsed: unknown
  try {
    parsed = JSON.parse(json)
  } catch {
    throw new Error('Workflow file is not valid JSON.')
  }

  if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) {
    throw new Error('Workflow file must be a JSON object.')
  }

  const workflow = { ...(parsed as Record<string, unknown>) }
  const id = workflow.id
  if (typeof id !== 'string' || !id.trim()) {
    throw new Error('Workflow ID is required.')
  }

  if (resolution === 'copy') {
    const copyId = nextCopyId(id)
    workflow.id = copyId
    workflow.name =
      typeof workflow.name === 'string' && workflow.name.trim()
        ? `${workflow.name} Copy`
        : copyId
  }

  return {
    id: workflow.id as string,
    content: `${JSON.stringify(workflow, null, 2)}\n`,
  }
}

async function readImportedWorkflow(file: File) {
  return file.text()
}

async function loadImportedWorkflow(id: string) {
  if (currentId.value === id) {
    store.clearSelection()
    await store.selectWorkflow(id)
    return
  }

  await store.requestSelectWorkflow(id)
}

async function importWorkflow(json: string, resolution?: 'overwrite' | 'copy') {
  const prepared = prepareImportedWorkflow(json, resolution)
  const existing = summaries.value.find((workflow) => workflow.id === prepared.id)

  if (existing && !resolution) {
    importConflict.value = { id: existing.id, json, name: existing.name }
    return
  }

  if (!pipelineStore.connected) {
    await pipelineStore.connect()
  }

  await workflowMcp.callTool('SaveWorkflowAsset', {
    id: prepared.id,
    content: prepared.content,
  })
  clearImportState()
  await store.fetchSummaries()
  await loadImportedWorkflow(prepared.id)
}

async function handleImportClick() {
  importInputRef.value?.click()
}

async function handleImportSelection(event: Event) {
  clearImportState()
  const input = event.target as HTMLInputElement | null
  const file = input?.files?.[0]

  if (!file) {
    return
  }

  try {
    const json = await readImportedWorkflow(file)
    await importWorkflow(json)
  } catch (error) {
    importError.value = error instanceof Error ? error.message : String(error)
  } finally {
    if (input) {
      input.value = ''
    }
  }
}

async function handleConflictResolution(resolution: 'overwrite' | 'copy') {
  if (!importConflict.value) return
  try {
    await importWorkflow(importConflict.value.json, resolution)
  } catch (error) {
    importError.value = error instanceof Error ? error.message : String(error)
  }
}

async function handleRetry() {
  clearImportState()
  if (!pipelineStore.connected) await pipelineStore.connect()
  await store.fetchSummaries()
}
</script>

<template>
  <aside class="workflow-browser">
    <input
      ref="importInputRef"
      class="sr-only-input"
      type="file"
      accept=".json,application/json"
      @change="handleImportSelection"
    />
    <div class="browser-header">
      <span class="header-title">WORKFLOW BROWSER</span>
    </div>

    <div class="search-wrapper">
      <svg class="search-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="11" cy="11" r="8" />
        <line x1="21" y1="21" x2="16.65" y2="16.65" />
      </svg>
      <input
        v-model="searchQuery"
        type="text"
        class="search-input"
        placeholder="Find workflow..."
      />
    </div>

    <div class="chip-row">
      <button
        v-for="chip in chips"
        :key="chip.value"
        :class="['chip', { 'chip--active': categoryFilter === chip.value }]"
        type="button"
        @click="categoryFilter = chip.value"
      >
        {{ chip.label }}
      </button>
    </div>

    <div v-if="browserError" class="import-feedback import-feedback--error" role="alert">
      <span class="feedback-text">{{ browserError }}</span>
      <button class="feedback-btn" type="button" @click="handleRetry">
        Retry
      </button>
    </div>

    <div
      v-if="importConflict || importError"
      class="import-feedback"
      :class="{ 'import-feedback--error': importError }"
      :role="importError ? 'alert' : 'status'"
      aria-live="polite"
    >
      <template v-if="importConflict">
        <span class="feedback-text">
          Workflow ID <code>{{ importConflict.id }}</code> already exists ({{ importConflict.name }}).
        </span>
        <button class="feedback-btn" type="button" @click="handleConflictResolution('overwrite')">
          Overwrite
        </button>
        <button class="feedback-btn" type="button" @click="handleConflictResolution('copy')">
          Import as Copy
        </button>
        <button class="feedback-btn" type="button" @click="clearImportState()">
          Cancel
        </button>
      </template>
      <template v-else-if="importError">
        <span class="feedback-text">{{ importError }}</span>
        <button class="feedback-btn" type="button" @click="importError = null">
          Dismiss
        </button>
      </template>
    </div>

    <ul class="workflow-list">
      <li
        v-for="wf in filteredWorkflows"
        :key="wf.id"
        :class="[
          'workflow-item',
          {
            'workflow-item--dirty': wf.id === currentId && isDirty,
            'workflow-item--selected': wf.id === currentId,
          },
        ]"
        @click="store.requestSelectWorkflow(wf.id)"
      >
        <span class="workflow-emoji">{{ wf.emoji }}</span>
        <div class="workflow-info">
          <span class="workflow-name">{{ wf.name }}</span>
          <div class="workflow-badges">
            <span v-if="wf.multiAgent" class="badge-multi-agent">MULTI-AGENT</span>
          </div>
        </div>
      </li>

      <li v-if="filteredWorkflows.length === 0" class="empty-state">
        No workflows found
      </li>
    </ul>

    <div class="browser-toolbar">
      <button class="btn-new" type="button" @click="store.createNewWorkflow()">
        + New Workflow
      </button>
      <button class="btn-import" type="button" title="Import workflow" @click="handleImportClick()">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
          <polyline points="7 10 12 15 17 10" />
          <line x1="12" y1="15" x2="12" y2="3" />
        </svg>
      </button>
    </div>
  </aside>
</template>

<style scoped>
.sr-only-input {
  display: none;
}
.workflow-browser { width: var(--sidebar-width); min-width: var(--sidebar-min-width); max-width: var(--sidebar-max-width); height: 100%; background: var(--bg-panel); display: flex; flex-direction: column; overflow: hidden; }
.browser-header { min-height: 44px; padding: 0 var(--space-4); border-bottom: 1px solid var(--border-default); display: flex; align-items: center; flex-shrink: 0; }
.header-title { display: inline-flex; align-items: center; gap: var(--space-2); font-size: 0; color: transparent; }
.header-title::before { content: ''; width: 6px; height: 6px; border-radius: 50%; background: var(--color-node-workflow); }
.header-title::after { content: 'WORKFLOWS'; font: 600 var(--text-xs)/1 var(--font-mono); letter-spacing: 0.06em; text-transform: uppercase; color: var(--text-secondary); }
.search-wrapper { position: relative; padding: var(--space-4) var(--space-4) var(--space-3); }
.search-icon { position: absolute; left: calc(var(--space-4) + var(--space-3)); top: calc(50% + var(--space-1) / 2); transform: translateY(-50%); color: var(--text-muted); pointer-events: none; }
.search-input { width: 100%; padding: var(--space-2) var(--space-3) var(--space-2) calc(var(--space-3) + 20px); background: var(--bg-input); border: 1px solid var(--border-default); border-radius: var(--radius-md); color: var(--text-primary); font: 400 var(--text-sm)/1.4 var(--font-sans); outline: none; transition: border-color var(--transition-fast), box-shadow var(--transition-fast), background var(--transition-fast); }
.search-input::placeholder { color: var(--text-muted); }
.search-input:hover:not(:focus) { border-color: color-mix(in srgb, var(--border-default) 130%, transparent); }
.search-input:focus { border-color: var(--border-focus); box-shadow: 0 0 0 2px color-mix(in srgb, var(--border-focus) 20%, transparent); }
.chip-row { display: flex; gap: var(--space-2); padding: 0 var(--space-4) var(--space-4); overflow-x: auto; scrollbar-width: none; flex-shrink: 0; }
.chip-row::-webkit-scrollbar { display: none; }
.chip { flex-shrink: 0; padding: var(--space-1) var(--space-2); border: 1px solid transparent; border-radius: var(--radius-sm); background: var(--surface-container); color: var(--text-secondary); font: 600 10px/1.4 var(--font-mono); letter-spacing: 0.05em; text-transform: uppercase; cursor: pointer; white-space: nowrap; transition: background var(--transition-fast), border-color var(--transition-fast), color var(--transition-fast); }
.chip:hover { background: var(--bg-surface); color: var(--text-primary); }
.chip--active { background: color-mix(in srgb, var(--color-node-workflow) 15%, var(--bg-surface)); color: var(--color-node-workflow); border-color: color-mix(in srgb, var(--color-node-workflow) 40%, transparent); }
.import-feedback { display: flex; flex-wrap: wrap; gap: var(--space-2); align-items: center; margin: 0 var(--space-4) var(--space-3); padding: var(--space-3); border: 1px solid var(--border-default); border-radius: var(--radius-md); background: var(--surface-container); }
.import-feedback--error { border-color: color-mix(in srgb, var(--color-status-abandoned) 45%, transparent); color: var(--color-status-abandoned); }
.feedback-text { flex: 1 1 100%; font: 500 var(--text-xs)/1.4 var(--font-sans); color: inherit; word-break: break-word; }
.feedback-btn { min-height: 24px; padding: 0 var(--space-2); border: 1px solid var(--border-default); border-radius: var(--radius-sm); background: transparent; color: var(--text-primary); font: 500 var(--text-xs)/1 var(--font-sans); cursor: pointer; }
.feedback-btn:hover { background: var(--bg-surface); }
.workflow-list { flex: 1; list-style: none; margin: 0; padding: 0 var(--space-3) var(--space-4); overflow-y: auto; display: flex; flex-direction: column; gap: var(--space-2); scrollbar-width: thin; scrollbar-color: var(--surface-container) transparent; }
.workflow-item { display: flex; align-items: flex-start; gap: var(--space-3); padding: var(--space-3); border: 1px solid var(--border-default); border-left: 4px solid transparent; border-radius: var(--radius-md); background: var(--surface-container); cursor: pointer; transition: background var(--transition-fast), border-color var(--transition-fast), box-shadow var(--transition-fast); }
.workflow-item:hover { background: var(--bg-surface); }
.workflow-item--selected { background: var(--bg-surface); border-left-color: var(--color-node-workflow); border-color: color-mix(in srgb, var(--color-node-workflow) 30%, var(--border-default)); box-shadow: 0 0 0 1px color-mix(in srgb, var(--color-node-workflow) 18%, transparent); }
.workflow-item--dirty::before { content: ''; align-self: center; width: 4px; height: 4px; border-radius: 50%; background: var(--color-status-active); margin-right: calc(var(--space-1) * -1); }
.workflow-emoji { font-size: var(--text-base); line-height: 1.25; flex-shrink: 0; }
.workflow-info { display: flex; flex-direction: column; gap: var(--space-1); min-width: 0; }
.workflow-name { font: 600 var(--text-sm)/1.4 var(--font-sans); color: var(--text-primary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.workflow-badges { display: flex; gap: var(--space-1); flex-wrap: wrap; }
.badge-multi-agent { display: inline-flex; align-items: center; padding: 1px var(--space-1); border-radius: var(--radius-sm); background: color-mix(in srgb, var(--color-node-workflow) 12%, var(--bg-panel)); color: var(--color-node-workflow); font: 600 10px/1.5 var(--font-mono); letter-spacing: 0.03em; text-transform: uppercase; }
.empty-state { padding: var(--space-8) var(--space-4); display: flex; flex-direction: column; align-items: center; gap: var(--space-2); text-align: center; color: transparent; font-size: 0; }
.empty-state::before { content: '⌕'; font-size: 24px; line-height: 1; color: color-mix(in srgb, var(--text-muted) 70%, transparent); }
.empty-state::after { content: 'Nothing found\A Try a different search'; white-space: pre-line; font-family: var(--font-sans); font-size: var(--text-sm); line-height: 1.6; color: var(--text-muted); }
.browser-toolbar { display: flex; gap: var(--space-2); padding: var(--space-3) var(--space-4) var(--space-4); border-top: 1px solid var(--border-default); flex-shrink: 0; }
.btn-new, .btn-import { min-height: 36px; border-radius: var(--radius-md); cursor: pointer; transition: background var(--transition-fast), border-color var(--transition-fast), color var(--transition-fast), box-shadow var(--transition-fast); }
.btn-new { flex: 1; padding: var(--space-2) var(--space-3); border: 1px solid var(--color-node-workflow); background: transparent; color: var(--color-node-workflow); font: 600 var(--text-sm)/1.2 var(--font-sans); }
.btn-new:hover { background: color-mix(in srgb, var(--color-node-workflow) 12%, transparent); box-shadow: 0 0 0 1px color-mix(in srgb, var(--color-node-workflow) 16%, transparent); }
.btn-import { display: grid; place-items: center; width: 36px; border: 1px solid var(--border-default); background: transparent; color: var(--text-secondary); }
.btn-import:hover { background: var(--surface-container); border-color: color-mix(in srgb, var(--color-node-workflow) 24%, var(--border-default)); color: var(--text-primary); }
</style>
