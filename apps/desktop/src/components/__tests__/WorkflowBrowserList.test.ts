// @vitest-environment jsdom
import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it } from 'vitest'
import { nextTick } from 'vue'

import WorkflowBrowserList from '../WorkflowBrowserList.vue'
import type { WorkflowAssetSummary } from '../../lib/workflow-asset-types'

const workflows: WorkflowAssetSummary[] = [
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

function mountList() {
  return mount(WorkflowBrowserList, {
    attachTo: document.body,
    props: {
      currentId: 'reasoning-loop',
      filteredWorkflows: workflows,
      isDirty: false,
      stepCounts: { 'reasoning-loop': 2, 'agent-review': 1 },
    },
  })
}

function dialogButton(label: string): HTMLButtonElement {
  const button = Array.from(document.body.querySelectorAll('button'))
    .find((candidate) => candidate.textContent?.trim() === label)
  if (!(button instanceof HTMLButtonElement)) {
    throw new Error(`Dialog button not found: ${label}`)
  }
  return button
}

describe('WorkflowBrowserList actions', () => {
  beforeEach(() => {
    document.body.innerHTML = ''
  })

  it('renders duplicate and delete actions per row', () => {
    const wrapper = mountList()

    expect(wrapper.findAll('button[aria-label^="Duplicate"]').length).toBe(2)
    expect(wrapper.findAll('button[aria-label^="Delete"]').length).toBe(2)

    wrapper.unmount()
  })

  it('duplicate emits selected workflow id', async () => {
    const wrapper = mountList()

    await wrapper.get('button[aria-label="Duplicate Agent Review"]').trigger('click')

    expect(wrapper.emitted('duplicate')?.[0]).toEqual(['agent-review'])
    wrapper.unmount()
  })

  it('delete opens confirmation dialog', async () => {
    const wrapper = mountList()

    await wrapper.get('button[aria-label="Delete Reasoning Loop"]').trigger('click')
    await nextTick()

    expect(document.body.textContent).toContain('Delete workflow Reasoning Loop?')
    expect(document.body.textContent).toContain('This will permanently delete the workflow asset.')
    expect(dialogButton('Delete').className).toContain('text-danger')
    wrapper.unmount()
  })

  it('delete confirm emits selected workflow id', async () => {
    const wrapper = mountList()

    await wrapper.get('button[aria-label="Delete Agent Review"]').trigger('click')
    await nextTick()
    dialogButton('Delete').click()
    await nextTick()

    expect(wrapper.emitted('delete')?.[0]).toEqual(['agent-review'])
    wrapper.unmount()
  })

  it('delete cancel closes dialog without emitting delete', async () => {
    const wrapper = mountList()

    await wrapper.get('button[aria-label="Delete Agent Review"]').trigger('click')
    await nextTick()
    dialogButton('Cancel').click()
    await nextTick()

    expect(wrapper.emitted('delete')).toBeUndefined()
    expect(document.body.textContent).not.toContain('Delete workflow Agent Review?')
    wrapper.unmount()
  })
})
