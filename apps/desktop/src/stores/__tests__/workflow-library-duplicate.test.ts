// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { ref } from 'vue'
import type {
  WorkflowAssetDefinition,
  WorkflowAssetSummary,
} from '../../lib/workflow-asset-types'
import { useWorkflowLibraryStore } from '../workflow-library'

const { toast } = vi.hoisted(() => ({
  toast: { error: vi.fn() },
}))
vi.mock('vue-sonner', () => ({ toast }))

const workflowSummaries: WorkflowAssetSummary[] = [
  makeSummary('my-flow', 'My Flow', 'reasoning'),
  makeSummary('my-flow-copy', 'My Flow Copy', 'reasoning'),
]

const connected = ref(true)
const callTool = vi.fn(async () => null)
const listWorkflowSummaries = vi.fn(async () => workflowSummaries)
const readWorkflowDefinition = vi.fn(async (id: string) => makeDefinition(id))

vi.mock('../../composables/usePipelineMcp', () => ({
  usePipelineMcp: () => ({
    connected,
    error: ref(null),
    connect: vi.fn(async () => {}),
    callTool,
    listWorkflowSummaries,
    readWorkflowDefinition,
  }),
}))

function makeSummary(
  id: string,
  name: string,
  category: WorkflowAssetSummary['category'],
): WorkflowAssetSummary {
  return { id, name, category, emoji: '⚙️', description: `${name} description`, multiAgent: false }
}

function makeDefinition(id: string): WorkflowAssetDefinition {
  return {
    id,
    name: id === 'my-flow' ? 'My Flow' : id,
    category: 'reasoning',
    emoji: '⚙️',
    description: '',
    sequentialThinkingEnabled: false,
    multiAgent: false,
    triggers: [],
    steps: [{ id: `${id}-step`, name: `${id} step`, description: 'A step.', requiresSequentialThinking: false }],
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
  callTool.mockClear()
  listWorkflowSummaries.mockClear()
  readWorkflowDefinition.mockClear()
  toast.error.mockClear()
})

describe('duplicateCurrentWorkflow', () => {
  it('avoids id collision: uses -copy-2 when -copy already exists', async () => {
    const store = await createLoadedStore()
    await store.selectWorkflow('my-flow')

    const newId = await store.duplicateCurrentWorkflow()

    expect(newId).toBe('my-flow-copy-2')
  })

  it('uses -copy suffix when no collision exists', async () => {
    listWorkflowSummaries.mockResolvedValueOnce([makeSummary('solo', 'Solo', 'reasoning')])
    setActivePinia(createPinia())
    const store = useWorkflowLibraryStore()
    await store.fetchSummaries()
    await store.selectWorkflow('solo')

    const newId = await store.duplicateCurrentWorkflow()

    expect(newId).toBe('solo-copy')
  })

  it('calls SaveWorkflowAsset with the new id', async () => {
    const store = await createLoadedStore()
    await store.selectWorkflow('my-flow')

    await store.duplicateCurrentWorkflow()

    expect(callTool).toHaveBeenCalledWith(
      'SaveWorkflowAsset',
      expect.objectContaining({ id: 'my-flow-copy-2' }),
    )
  })

  it('calls fetchSummaries and auto-selects the new workflow', async () => {
    listWorkflowSummaries
      .mockResolvedValueOnce([makeSummary('solo', 'Solo', 'reasoning')])
      .mockResolvedValueOnce([makeSummary('solo', 'Solo', 'reasoning')])
      .mockResolvedValueOnce([
        makeSummary('solo', 'Solo', 'reasoning'),
        makeSummary('solo-copy', 'Solo Copy', 'reasoning'),
      ])
    setActivePinia(createPinia())
    const store = useWorkflowLibraryStore()
    await store.fetchSummaries()
    await store.selectWorkflow('solo')

    listWorkflowSummaries.mockClear()
    await store.duplicateCurrentWorkflow()

    expect(listWorkflowSummaries).toHaveBeenCalledTimes(1)
    expect(store.currentId).toBe('solo-copy')
  })

  it('surfaces error and returns null when SaveWorkflowAsset rejects', async () => {
    const store = await createLoadedStore()
    await store.selectWorkflow('my-flow')
    callTool.mockRejectedValueOnce(new Error('disk full'))
    listWorkflowSummaries.mockClear()

    const result = await store.duplicateCurrentWorkflow()

    expect(result).toBeNull()
    expect(toast.error).toHaveBeenCalledWith('disk full')
    expect(listWorkflowSummaries).not.toHaveBeenCalled()
  })

  it('returns copyId when fetchSummaries fails internally after save', async () => {
    const store = await createLoadedStore()
    await store.selectWorkflow('my-flow')
    listWorkflowSummaries.mockClear()
    listWorkflowSummaries.mockRejectedValueOnce(new Error('network error'))

    const result = await store.duplicateCurrentWorkflow()

    expect(result).toBe('my-flow-copy-2')
    expect(callTool).toHaveBeenCalledWith('SaveWorkflowAsset', expect.objectContaining({ id: 'my-flow-copy-2' }))
    expect(toast.error).not.toHaveBeenCalled()
  })

  it('returns null immediately when currentId is null', async () => {
    setActivePinia(createPinia())
    const store = useWorkflowLibraryStore()
    callTool.mockClear()

    const result = await store.duplicateCurrentWorkflow()

    expect(result).toBeNull()
    expect(callTool).not.toHaveBeenCalled()
  })
})
