import { computed, type ComputedRef, type Ref } from 'vue'
import type { WorkflowAssetSummary, WorkflowCategory } from '../lib/workflow-asset-types'
import { categorizeWorkflow } from '../lib/workflow-asset-types'

interface WorkflowLibrarySelectionOptions {
  summaries: Ref<WorkflowAssetSummary[]>
  searchQuery: Ref<string>
  categoryFilter: Ref<WorkflowCategory | 'all'>
  currentId: Ref<string | null>
  pendingWorkflowId: Ref<string | null>
  isDirty: ComputedRef<boolean>
  selectWorkflow: (id: string) => Promise<void>
}

export function createWorkflowLibrarySelection({
  summaries,
  searchQuery,
  categoryFilter,
  currentId,
  pendingWorkflowId,
  isDirty,
  selectWorkflow,
}: WorkflowLibrarySelectionOptions) {
  const filteredWorkflows = computed(() => {
    let result = summaries.value
    if (categoryFilter.value !== 'all') {
      result = result.filter((wf) => categorizeWorkflow(wf) === categoryFilter.value)
    }
    const q = searchQuery.value.trim().toLowerCase()
    if (q.length > 0) {
      result = result.filter(
        (wf) =>
          wf.name.toLowerCase().includes(q) ||
          wf.description.toLowerCase().includes(q) ||
          categorizeWorkflow(wf).toLowerCase().includes(q),
      )
    }
    return result
  })

  async function ensureVisibleWorkflowSelected() {
    const visibleWorkflows = filteredWorkflows.value
    if (visibleWorkflows.length === 0) return
    if (
      currentId.value &&
      visibleWorkflows.some((workflow) => workflow.id === currentId.value)
    ) {
      return
    }
    if (isDirty.value && currentId.value) {
      pendingWorkflowId.value = visibleWorkflows[0].id
      return
    }
    await selectWorkflow(visibleWorkflows[0].id)
  }

  function setSearchQuery(value: string) {
    searchQuery.value = value
    void ensureVisibleWorkflowSelected()
  }

  function setCategoryFilter(value: WorkflowCategory | 'all') {
    categoryFilter.value = value
    void ensureVisibleWorkflowSelected()
  }

  return {
    filteredWorkflows,
    ensureVisibleWorkflowSelected,
    setSearchQuery,
    setCategoryFilter,
  }
}
