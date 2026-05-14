import { computed, ref, watch } from 'vue'
import { defineStore } from 'pinia'
import type { WorkflowAssetDefinition, WorkflowAssetSummary, WorkflowCategory } from '../lib/workflow-asset-types'
import { filterWorkflowSummaries, validateWorkflowDefinition } from '../lib/workflow-asset-runtime'
import { categorizeWorkflow } from '../lib/workflow-asset-types'
import { usePipelineStore } from './pipeline'
import { createWorkflowLibraryDraftActions } from './workflow-library-draft-actions'
import {
  deepClone,
  definitionsEqual,
  detectWorkflowOrigin,
  type WorkflowOrigin,
  type WorkflowRaw,
} from './workflow-library-helpers'
import {
  IMPORT_WORKFLOW_PENDING_ID,
  NEW_WORKFLOW_PENDING_ID,
  createWorkflowLibrarySessionActions,
  type PendingImportedWorkflow,
} from './workflow-library-session-actions'
import {
  createWorkflowLibrarySaveActions,
  type WorkflowExportState,
  type WorkflowSaveState,
} from './workflow-library-save-actions'
import { createWorkflowLibrarySelection } from './workflow-library-selection'
export const useWorkflowLibraryStore = defineStore('workflow-library', () => {
  const pipelineStore = usePipelineStore()
  const summaries = ref<WorkflowAssetSummary[]>([])
  const currentDefinition = ref<WorkflowAssetDefinition | null>(null)
  const draftDefinition = ref<WorkflowAssetDefinition | null>(null)
  const rawJson = ref<WorkflowRaw | null>(null)
  const currentId = ref<string | null>(null)
  const pendingWorkflowId = ref<string | null>(null)
  const pendingImportedWorkflow = ref<PendingImportedWorkflow | null>(null)
  const origin = ref<WorkflowOrigin | null>(null)
  const loading = ref(false)
  const definitionLoading = ref(false)
  const error = ref<string | null>(null)
  const exportError = ref<string | null>(null)
  const exportState = ref<WorkflowExportState>('idle')
  const saveError = ref<string | null>(null), saveState = ref<WorkflowSaveState>('idle')
  const searchQuery = ref(''), categoryFilter = ref<WorkflowCategory | 'all'>('all')
  const isDirty = computed(() => !definitionsEqual(draftDefinition.value, currentDefinition.value))
  const currentStepCount = computed(() => draftDefinition.value?.steps.length ?? 0), categories = computed(() => Array.from(new Set(summaries.value.map(categorizeWorkflow))).sort())
  const canDuplicate = computed(() => !!currentId.value && currentId.value !== NEW_WORKFLOW_PENDING_ID && currentId.value !== IMPORT_WORKFLOW_PENDING_ID)

  async function fetchSummaries() {
    if (!pipelineStore.connected) return
    loading.value = true
    error.value = null
    try {
      const { valid, invalid } = filterWorkflowSummaries(await pipelineStore.listWorkflowSummaries())
      summaries.value = valid
      await ensureVisibleWorkflowSelected()
      if (invalid.length > 0) error.value = `Ignored invalid workflows: ${invalid.map(({ id, reason }) => `${id} (${reason})`).join('; ')}`
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loading.value = false
    }
  }

  function beginEditing() {
    if (!currentDefinition.value) return (draftDefinition.value = null)
    draftDefinition.value = deepClone(currentDefinition.value)
    resetSaveFeedback()
    return draftDefinition.value
  }

  function resetSaveFeedback() {
    if (saveState.value !== 'saving') saveState.value = 'idle'
    saveError.value = null
  }

  async function loadWorkflow(id: string) {
    currentId.value = id
    draftDefinition.value = null
    definitionLoading.value = true
    error.value = null
    try {
      const payload = validateWorkflowDefinition(await pipelineStore.readWorkflowDefinition(id))
      currentDefinition.value = deepClone(payload)
      rawJson.value = deepClone(payload as unknown as WorkflowRaw)
      origin.value = detectWorkflowOrigin(rawJson.value, id)
      beginEditing()
      pendingWorkflowId.value = null
      pendingImportedWorkflow.value = null
      exportState.value = 'idle'
      exportError.value = null
      saveState.value = 'idle'
      saveError.value = null
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
      currentId.value = null
      currentDefinition.value = null
      rawJson.value = null
      origin.value = null
      exportError.value = null
      exportState.value = 'idle'
      saveError.value = null
      saveState.value = 'idle'
    } finally {
      definitionLoading.value = false
    }
  }
  async function selectWorkflow(id: string) {
    if (currentId.value === id && currentDefinition.value) return
    await loadWorkflow(id)
  }

  async function requestSelectWorkflow(id: string) {
    if (currentId.value === id && currentDefinition.value) return
    if (isDirty.value && currentId.value && currentId.value !== id) {
      pendingWorkflowId.value = id
      return
    }
    await loadWorkflow(id)
  }

  const {
    ensureVisibleWorkflowSelected,
    filteredWorkflows,
    setCategoryFilter,
    setSearchQuery,
  } = createWorkflowLibrarySelection({
    summaries,
    searchQuery,
    categoryFilter,
    currentId,
    pendingWorkflowId,
    isDirty,
    selectWorkflow,
  })

  async function confirmDiscardAndContinue() {
    const nextId = pendingWorkflowId.value
    pendingWorkflowId.value = null
    if (nextId === NEW_WORKFLOW_PENDING_ID) {
      discardDraft()
      createNewWorkflow(true)
      return
    }
    if (nextId === IMPORT_WORKFLOW_PENDING_ID) {
      clearSelection()
      applyPendingImport()
      return
    }
    discardDraft()
    if (nextId) {
      await loadWorkflow(nextId)
    }
  }

  function cancelPendingSelection() {
    pendingWorkflowId.value = null
    clearPendingImport()
  }

  function discardDraft() {
    if (currentDefinition.value) {
      beginEditing()
      return
    }
    clearSelection()
  }

  const {
    applyPendingImport,
    clearPendingImport,
    createNewWorkflow,
    importWorkflow,
  } = createWorkflowLibrarySessionActions({
    currentDefinition,
    currentId,
    draftDefinition,
    error,
    exportError,
    exportState,
    isDirty,
    origin,
    pendingImportedWorkflow,
    pendingWorkflowId,
    rawJson,
    saveError,
    saveState,
    summaries,
  })

  const {
    addStep,
    moveStepDown,
    moveStepUp,
    removeStep,
    setStepReasoningEffort,
    setStepRequiresSequentialThinking,
    updateStepDescription,
    updateStepName,
    updateWorkflowCategory,
    updateWorkflowDescription,
    updateWorkflowEmoji,
    updateWorkflowMultiAgent,
    updateWorkflowName,
    updateWorkflowSequentialThinkingEnabled,
    updateWorkflowTriggers,
  } = createWorkflowLibraryDraftActions({
    beginEditing,
    draftDefinition,
    onMutate: resetSaveFeedback,
    rawJson,
  })
  const { exportWorkflow, saveDraft, duplicateCurrentWorkflow, duplicateWorkflowById } = createWorkflowLibrarySaveActions({
    applyPendingImport,
    createNewWorkflow,
    currentDefinition,
    currentId,
    draftDefinition,
    exportError,
    exportState,
    fetchSummaries,
    loadWorkflow,
    origin,
    pendingWorkflowId,
    rawJson,
    saveError, saveState,
    summaries,
    selectWorkflow,
  })

  function clearSelection() {
    currentId.value = null
    currentDefinition.value = null
    draftDefinition.value = null
    rawJson.value = null
    pendingWorkflowId.value = null
    pendingImportedWorkflow.value = null
    origin.value = null
    exportError.value = null
    exportState.value = 'idle'
    saveError.value = null
    saveState.value = 'idle'
  }

  watch(() => pipelineStore.connected, (connected) => {
    if (connected && summaries.value.length === 0) fetchSummaries()
  }, { immediate: true })

  return {
    summaries,
    currentDefinition,
    draftDefinition,
    rawJson,
    currentId,
    pendingWorkflowId,
    origin,
    loading,
    definitionLoading,
    error,
    exportError,
    exportState,
    saveError, saveState,
    searchQuery, categoryFilter,
    filteredWorkflows,
    isDirty,
    currentStepCount,
    categories,
    fetchSummaries,
    beginEditing,
    createNewWorkflow,
    importWorkflow,
    selectWorkflow,
    requestSelectWorkflow,
    ensureVisibleWorkflowSelected,
    confirmDiscardAndContinue,
    cancelPendingSelection,
    discardDraft,
    addStep,
    removeStep,
    moveStepUp,
    moveStepDown,
    updateStepName,
    updateStepDescription,
    setStepRequiresSequentialThinking,
    setStepReasoningEffort,
    updateWorkflowName,
    updateWorkflowEmoji,
    updateWorkflowDescription,
    updateWorkflowTriggers,
    updateWorkflowCategory,
    updateWorkflowSequentialThinkingEnabled,
    updateWorkflowMultiAgent,
    exportWorkflow, saveDraft,
    duplicateCurrentWorkflow, duplicateWorkflowById, canDuplicate,
    clearSelection,
    setSearchQuery,
    setCategoryFilter,
  }
})
