// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { ref } from 'vue'
import type {
  WorkflowAssetDefinition,
  WorkflowAssetSummary,
} from '../../lib/workflow-asset-types'
import { useWorkflowLibraryStore } from '../workflow-library'

const workflowSummaries: WorkflowAssetSummary[] = [
  makeSummary('reasoning-a', 'Reasoning A', 'reasoning'),
  makeSummary('scientific-a', 'Scientific A', 'scientific'),
  makeSummary('scientific-b', 'Scientific B', 'scientific'),
]

const connected = ref(true)
const listWorkflowSummaries = vi.fn(async () => workflowSummaries)
const readWorkflowDefinition = vi.fn(async (id: string) => makeDefinition(id))

vi.mock('../../composables/usePipelineMcp', () => ({
  usePipelineMcp: () => ({
    connected,
    error: ref(null),
    connect: vi.fn(async () => {}),
    listWorkflowSummaries,
    readWorkflowDefinition,
  }),
}))

function makeSummary(
  id: string,
  name: string,
  category: WorkflowAssetSummary['category'],
): WorkflowAssetSummary {
  return {
    id,
    name,
    category,
    emoji: '⚙️',
    description: `${name} description`,
    multiAgent: false,
  }
}

function makeDefinition(id: string): WorkflowAssetDefinition {
  const summary = workflowSummaries.find((workflow) => workflow.id === id)
  return {
    id,
    name: summary?.name ?? id,
    category: summary?.category,
    emoji: '⚙️',
    description: `${id} description`,
    sequentialThinkingEnabled: false,
    multiAgent: false,
    triggers: [],
    steps: [
      {
        id: `${id}-step`,
        name: `${id} step`,
        description: 'Run the workflow step.',
        requiresSequentialThinking: false,
      },
    ],
  }
}

async function createLoadedStore() {
  setActivePinia(createPinia())
  const store = useWorkflowLibraryStore()
  await store.fetchSummaries()
  return store
}

beforeEach(() => {
  connected.value = true
  listWorkflowSummaries.mockClear()
  readWorkflowDefinition.mockClear()
})

describe('useWorkflowLibraryStore visible selection', () => {
  it('selects the first visible workflow when none is selected', async () => {
    const store = await createLoadedStore()

    await store.ensureVisibleWorkflowSelected()

    expect(store.currentId).toBe('reasoning-a')
  })

  it('falls back to the first remaining workflow when filters hide the current selection', async () => {
    const store = await createLoadedStore()
    await store.selectWorkflow('reasoning-a')

    store.setCategoryFilter('scientific')
    await store.ensureVisibleWorkflowSelected()

    expect(store.currentId).toBe('scientific-a')
  })

  it('keeps the current selection when it remains visible after filtering', async () => {
    const store = await createLoadedStore()
    await store.selectWorkflow('scientific-b')

    store.setCategoryFilter('scientific')
    await store.ensureVisibleWorkflowSelected()

    expect(store.currentId).toBe('scientific-b')
  })

  it('stages a pending workflow when auto-selection would replace a dirty draft', async () => {
    const store = await createLoadedStore()
    await store.selectWorkflow('reasoning-a')
    store.updateWorkflowDescription('Unsaved reasoning changes')

    store.setCategoryFilter('scientific')
    await store.ensureVisibleWorkflowSelected()

    expect(store.currentId).toBe('reasoning-a')
    expect(store.pendingWorkflowId).toBe('scientific-a')
  })

  it('keeps the current selection while filteredWorkflows excludes it when search has no matches', async () => {
    const store = await createLoadedStore()
    await store.selectWorkflow('reasoning-a')

    store.setSearchQuery('no workflow matches this query')
    await store.ensureVisibleWorkflowSelected()

    expect(store.currentId).toBe('reasoning-a')
    expect(store.filteredWorkflows).toHaveLength(0)
    expect(store.filteredWorkflows.find((workflow) => workflow.id === 'reasoning-a')).toBeUndefined()
  })
})
