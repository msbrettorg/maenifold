<script setup lang="ts">
import { toastSuccess, toastError } from '@/composables/useToast';
import { storeToRefs } from 'pinia'
import { computed, ref, watch } from 'vue'
import { Download, Plus } from 'lucide-vue-next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ScrollArea } from '@/components/ui/scroll-area'
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
const stepCounts = ref<Record<string, number>>({})

interface CategoryChip {
  label: string
  value: WorkflowCategory | 'all'
}

const chips: CategoryChip[] = [
  { label: 'All', value: 'all' },
  { label: 'Reasoning', value: 'reasoning' },
  { label: 'Scientific', value: 'scientific' },
  { label: 'Multi-agent', value: 'multi-agent' },
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

  await workflowMcp.gatewayExecute('maenifold.workflow', 'SaveWorkflowAsset', {
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

async function handleDuplicateWorkflow(id: string) {
  await store.duplicateWorkflowById(id)
}

async function handleDeleteWorkflow(id: string) {
  try {
    if (!workflowMcp.connected.value) {
      await workflowMcp.connect()
    }
    await workflowMcp.gatewayExecute('maenifold.workflow', 'DeleteWorkflowAsset', { name: id })
    toastSuccess('Workflow deleted.')
    await store.fetchSummaries()
    if (currentId.value === id) {
      const nextWorkflow = filteredWorkflows.value.find((workflow) => workflow.id !== id)
      if (nextWorkflow) {
        await store.selectWorkflow(nextWorkflow.id)
      } else {
        store.clearSelection()
      }
    }
  } catch (error) {
    toastError(error instanceof Error ? error.message : String(error))
  }
}

async function loadVisibleStepCounts() {
  if (!pipelineStore.connected) return
  const pendingIds = filteredWorkflows.value
    .map((workflow) => workflow.id)
    .filter((id) => stepCounts.value[id] === undefined)

  if (pendingIds.length === 0) return

  const results = await Promise.allSettled(
    pendingIds.map(async (id) => [id, (await workflowMcp.readWorkflowDefinition(id)).steps.length] as const),
  )

  const nextCounts = { ...stepCounts.value }
  for (const result of results) {
    if (result.status === 'fulfilled') {
      const [id, count] = result.value
      nextCounts[id] = count
    }
  }
  stepCounts.value = nextCounts
}

watch(
  () => filteredWorkflows.value.map((workflow) => workflow.id).join('|'),
  () => {
    void loadVisibleStepCounts()
  },
  { immediate: true },
)
</script>

<template>
  <aside
    class="flex h-full w-[22rem] min-w-80 max-w-[22rem] flex-col overflow-hidden bg-bg-sidebar"
    role="region"
    aria-label="Workflow library"
  >
    <Input
      ref="importInputRef"
      class="hidden"
      type="file"
      accept=".json,application/json"
      aria-hidden="true"
      tabindex="-1"
      @change="handleImportSelection"
    />

    <WorkflowBrowserFilters
      :chips="chips"
      :search-query="searchQuery"
      :category-filter="categoryFilter"
      :browser-error="browserError"
      :import-error="importError"
      :import-conflict="importConflict"
      @update:search-query="store.setSearchQuery"
      @update:category-filter="store.setCategoryFilter"
      @retry="handleRetry"
      @resolve-conflict="handleConflictResolution"
      @clear-import-state="clearImportState"
      @dismiss-import-error="importError = null"
    />

    <ScrollArea class="min-h-0 flex-1">
      <WorkflowBrowserList
        :current-id="currentId"
        :filtered-workflows="filteredWorkflows"
        :is-dirty="isDirty"
        :step-counts="stepCounts"
        @select="store.requestSelectWorkflow"
        @duplicate="handleDuplicateWorkflow"
        @delete="handleDeleteWorkflow"
      />
    </ScrollArea>

    <div class="flex shrink-0 gap-2 px-4 py-3 pb-4">
      <Button
        type="button"
        class="flex-1 gap-1.5"
        @click="store.createNewWorkflow()"
      >
        <Plus class="size-4" aria-hidden="true" />
        <span>New workflow</span>
      </Button>
      <Button
        type="button"
        variant="outline"
        size="icon"
        class="size-9 text-text-secondary hover:text-text-primary"
        title="Import workflow"
        aria-label="Import workflow"
        @click="handleImportClick()"
      >
        <Download class="size-4" aria-hidden="true" />
      </Button>
    </div>
  </aside>
</template>
