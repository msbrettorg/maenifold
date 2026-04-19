import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import type { WorkflowAssetSummary, WorkflowCategory } from '../lib/workflow-asset-types'
import { categorizeWorkflow } from '../lib/workflow-asset-types'
import { useWorkflowLibraryStore } from './workflow-library'

const STORAGE_KEY = 'workflow-browser-v1'
const CATEGORY_VALUES: Array<WorkflowCategory | 'all'> = [
  'all',
  'reasoning',
  'scientific',
  'multi-agent',
  'devops',
  'memory',
  'custom',
]

interface PersistedWorkflowBrowserState {
  searchQuery?: unknown
  categoryFilter?: unknown
}

function normalizeCategory(value: unknown): WorkflowCategory | 'all' {
  if (typeof value !== 'string') return 'all'
  return CATEGORY_VALUES.includes(value as WorkflowCategory | 'all')
    ? (value as WorkflowCategory | 'all')
    : 'all'
}

function readPersistedState(): PersistedWorkflowBrowserState {
  if (typeof window === 'undefined') return {}
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY)
    return raw ? (JSON.parse(raw) as PersistedWorkflowBrowserState) : {}
  } catch {
    return {}
  }
}

function categoryLabel(category: WorkflowCategory | 'all') {
  if (category === 'all') return 'All'
  if (category === 'multi-agent') return 'Multi-agent'
  if (category === 'devops') return 'DevOps'
  return category.charAt(0).toUpperCase() + category.slice(1)
}

function matchesSearch(workflow: WorkflowAssetSummary, query: string) {
  if (!query) return true
  const haystack = `${workflow.name} ${workflow.description} ${workflow.id}`.toLowerCase()
  return haystack.includes(query)
}

export const useWorkflowBrowserStore = defineStore('workflow-browser', () => {
  const workflowLibraryStore = useWorkflowLibraryStore()
  const persisted = readPersistedState()

  const searchQuery = ref(
    typeof persisted.searchQuery === 'string' ? persisted.searchQuery : '',
  )
  const categoryFilter = ref<WorkflowCategory | 'all'>(
    normalizeCategory(persisted.categoryFilter),
  )

  const filteredWorkflows = computed(() => {
    const query = searchQuery.value.trim().toLowerCase()
    return workflowLibraryStore.summaries.filter((workflow) => {
      if (
        categoryFilter.value !== 'all'
        && categorizeWorkflow(workflow) !== categoryFilter.value
      ) {
        return false
      }
      return matchesSearch(workflow, query)
    })
  })

  const categoryChips = computed(() =>
    CATEGORY_VALUES.map((value) => {
      const count =
        value === 'all'
          ? workflowLibraryStore.summaries.length
          : workflowLibraryStore.summaries.filter((workflow) => categorizeWorkflow(workflow) === value).length
      return {
        label: categoryLabel(value),
        value,
        count,
      }
    }),
  )

  function persistState() {
    if (typeof window === 'undefined') return
    try {
      window.localStorage.setItem(
        STORAGE_KEY,
        JSON.stringify({
          searchQuery: searchQuery.value,
          categoryFilter: categoryFilter.value,
        }),
      )
    } catch {
      // localStorage is best-effort only.
    }
  }

  function setSearchQuery(value: string) {
    searchQuery.value = value
    persistState()
  }

  function setCategoryFilter(value: WorkflowCategory | 'all') {
    categoryFilter.value = value
    persistState()
  }

  return {
    searchQuery,
    categoryFilter,
    filteredWorkflows,
    categoryChips,
    setSearchQuery,
    setCategoryFilter,
  }
})
