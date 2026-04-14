<script setup lang="ts">
import { storeToRefs } from 'pinia'
import { computed, ref } from 'vue'
import WorkflowBrowserFilters from './WorkflowBrowserFilters.vue'
import WorkflowBrowserList from './WorkflowBrowserList.vue'
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
  <aside class="workflow-browser" role="region" aria-label="Workflow library">
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

    <WorkflowBrowserFilters
      :chips="chips"
      :search-query="searchQuery"
      :category-filter="categoryFilter"
      :browser-error="browserError"
      :import-error="importError"
      :import-conflict="importConflict"
      @update:search-query="searchQuery = $event"
      @update:category-filter="categoryFilter = $event"
      @retry="handleRetry"
      @resolve-conflict="handleConflictResolution"
      @clear-import-state="clearImportState"
      @dismiss-import-error="importError = null"
    />

    <WorkflowBrowserList
      :current-id="currentId"
      :filtered-workflows="filteredWorkflows"
      :is-dirty="isDirty"
      @select="store.requestSelectWorkflow"
    />

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
.workflow-browser { width: var(--sidebar-width); min-width: var(--sidebar-min-width); max-width: var(--sidebar-max-width); height: 100%; background: var(--card); display: flex; flex-direction: column; overflow: hidden; }
.browser-header { min-height: 44px; padding: 0 var(--space-4); border-bottom: 1px solid var(--border); display: flex; align-items: center; flex-shrink: 0; }
.header-title { display: inline-flex; align-items: center; gap: var(--space-2); font-size: 0; color: transparent; }
.header-title::before { content: ''; width: 6px; height: 6px; border-radius: 50%; background: var(--color-node-workflow); }
.header-title::after { content: 'WORKFLOWS'; font: var(--weight-title) var(--text-tiny)/1 var(--font-code); letter-spacing: 0.06em; text-transform: uppercase; color: var(--muted-foreground); }
.browser-toolbar { display: flex; gap: var(--space-2); padding: var(--space-3) var(--space-4) var(--space-4); border-top: 1px solid var(--border); flex-shrink: 0; }
.btn-new, .btn-import { min-height: 36px; border-radius: var(--radius-md); cursor: pointer; transition: background var(--transition-fast), border-color var(--transition-fast), color var(--transition-fast), box-shadow var(--transition-fast); }
.btn-new { flex: 1; padding: var(--space-2) var(--space-3); border: 1px solid var(--color-node-workflow); background: transparent; color: var(--color-node-workflow); font: var(--weight-title) var(--text-caption)/1.2 var(--font-body); }
.btn-new:hover { background: color-mix(in srgb, var(--color-node-workflow) 12%, transparent); box-shadow: 0 0 0 1px color-mix(in srgb, var(--color-node-workflow) 16%, transparent); }
.btn-import { display: grid; place-items: center; width: 36px; border: 1px solid var(--border); background: transparent; color: var(--muted-foreground); }
.btn-import:hover { background: var(--card); border-color: color-mix(in srgb, var(--color-node-workflow) 24%, var(--border)); color: var(--foreground); }
</style>
