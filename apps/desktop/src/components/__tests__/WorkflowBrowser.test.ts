// @vitest-environment jsdom
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'

import WorkflowBrowser from '../WorkflowBrowser.vue'
import { useWorkflowLibraryStore } from '../../stores/workflow-library'
import type { WorkflowAssetDefinition, WorkflowAssetSummary } from '../../lib/workflow-asset-types'

const mocks = vi.hoisted(() => ({
  callTool: vi.fn(),
  connect: vi.fn(),
  readWorkflowDefinition: vi.fn(),
  listWorkflowSummaries: vi.fn(),
  toastSuccess: vi.fn(),
  toastError: vi.fn(),
}))

vi.mock('../../composables/usePipelineMcp', () => ({
  usePipelineMcp: () => ({
    callTool: mocks.callTool,
    connect: mocks.connect,
    connected: ref(true),
    error: ref(null),
    listWorkflowSummaries: mocks.listWorkflowSummaries,
    readWorkflowDefinition: mocks.readWorkflowDefinition,
  }),
}))

vi.mock('vue-sonner', () => ({
  toast: {
    success: mocks.toastSuccess,
    error: mocks.toastError,
  },
}))

const summaries: WorkflowAssetSummary[] = [
  {
    id: 'reasoning-loop',
    name: 'Reasoning Loop',
    emoji: '🧠',
    description: 'Reason through a problem',
    multiAgent: false,
    category: 'reasoning',
  },
  {
    id: 'agent-review',
    name: 'Agent Review',
    emoji: '👥',
    description: 'Review with agents',
    multiAgent: true,
    category: 'multi-agent',
  },
]

function definition(id: string): WorkflowAssetDefinition {
  const summary = summaries.find((workflow) => workflow.id === id) ?? summaries[0]
  return {
    id,
    name: summary.name,
    emoji: summary.emoji,
    description: summary.description,
    triggers: [],
    sequentialThinkingEnabled: false,
    multiAgent: summary.multiAgent,
    category: summary.category,
    steps: [
      {
        id: `${id}-step`,
        name: 'Step',
        description: 'Run a step',
        requiresSequentialThinking: false,
      },
    ],
  }
}

async function mountBrowser() {
  const pinia = createPinia()
  setActivePinia(pinia)
  const wrapper = mount(WorkflowBrowser, {
    attachTo: document.body,
    global: { plugins: [pinia] },
  })
  await flushPromises()
  return { wrapper, store: useWorkflowLibraryStore() }
}

function dialogButton(label: string): HTMLButtonElement {
  const button = Array.from(document.body.querySelectorAll('button'))
    .find((candidate) => candidate.textContent?.trim() === label)
  if (!(button instanceof HTMLButtonElement)) {
    throw new Error(`Dialog button not found: ${label}`)
  }
  return button
}

describe('WorkflowBrowser actions', () => {
  beforeEach(() => {
    document.body.innerHTML = ''
    mocks.callTool.mockReset()
    mocks.connect.mockReset()
    mocks.readWorkflowDefinition.mockReset()
    mocks.listWorkflowSummaries.mockReset()
    mocks.toastSuccess.mockReset()
    mocks.toastError.mockReset()
    mocks.listWorkflowSummaries.mockResolvedValue(summaries)
    mocks.readWorkflowDefinition.mockImplementation((id: string) => Promise.resolve(definition(id)))
    mocks.callTool.mockResolvedValue({ data: true })
    mocks.connect.mockResolvedValue(undefined)
  })

  it('duplicate selects the row then calls store duplicate action', async () => {
    const { wrapper, store } = await mountBrowser()
    const selectSpy = vi.spyOn(store, 'selectWorkflow')
    const duplicateSpy = vi.spyOn(store, 'duplicateCurrentWorkflow').mockResolvedValue('agent-review-copy')

    await wrapper.get('button[aria-label="Duplicate Agent Review"]').trigger('click')
    await flushPromises()

    expect(selectSpy).toHaveBeenCalledWith('agent-review')
    expect(duplicateSpy).toHaveBeenCalled()
    wrapper.unmount()
  })

  it('delete confirm calls DeleteWorkflowAsset MCP', async () => {
    const { wrapper } = await mountBrowser()

    await wrapper.get('button[aria-label="Delete Agent Review"]').trigger('click')
    await flushPromises()
    dialogButton('Delete').click()
    await flushPromises()

    expect(mocks.callTool).toHaveBeenCalledWith('DeleteWorkflowAsset', { name: 'agent-review' })
    expect(mocks.toastSuccess).toHaveBeenCalledWith('Workflow deleted.')
    wrapper.unmount()
  })

  it('delete cancel closes dialog without calling MCP', async () => {
    const { wrapper } = await mountBrowser()

    await wrapper.get('button[aria-label="Delete Agent Review"]').trigger('click')
    await flushPromises()
    dialogButton('Cancel').click()
    await flushPromises()

    expect(mocks.callTool).not.toHaveBeenCalled()
    wrapper.unmount()
  })

  it('delete refused-by-references shows error toast', async () => {
    const error = new Error('Workflow is referenced by running pipeline(s): pipeline-1')
    mocks.callTool.mockRejectedValue(error)
    const { wrapper } = await mountBrowser()

    await wrapper.get('button[aria-label="Delete Reasoning Loop"]').trigger('click')
    await flushPromises()
    dialogButton('Delete').click()
    await flushPromises()

    expect(mocks.toastError).toHaveBeenCalledWith(error.message)
    wrapper.unmount()
  })
})
