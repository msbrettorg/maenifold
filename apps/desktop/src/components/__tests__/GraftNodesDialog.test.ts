// @vitest-environment jsdom
import { flushPromises, mount } from '@vue/test-utils';
import type { Component } from 'vue';
import { describe, expect, it } from 'vitest';
import GraftNodesDialog from '../GraftNodesDialog.vue';

const NO_UPSTREAM_EDGE = '__none__';

const dialogStubs = {
  Dialog: { template: '<div><slot /></div>', props: ['open'] },
  DialogClose: { template: '<div><slot /></div>', props: ['asChild'] },
  DialogContent: { template: '<div><slot /></div>', props: ['showCloseButton'] },
  DialogDescription: { template: '<p><slot /></p>' },
  DialogFooter: { template: '<div><slot /></div>' },
  DialogHeader: { template: '<div><slot /></div>' },
  DialogTitle: { template: '<h2><slot /></h2>' },
};

const selectStubs = {
  Select: {
    name: 'Select',
    props: ['modelValue'],
    emits: ['update:modelValue'],
    template: '<div data-testid="select-stub" :data-model-value="modelValue"><slot /></div>',
  },
  SelectContent: { template: '<div><slot /></div>' },
  SelectItem: {
    props: ['value'],
    template: '<div role="option" :data-value="value"><slot /></div>',
  },
  SelectTrigger: { template: '<button type="button" v-bind="$attrs"><slot /></button>' },
  SelectValue: { props: ['placeholder'], template: '<span>{{ placeholder }}</span>' },
};

function mountDialog(sourceNodeId?: string) {
  const props = {
    existingNodeIds: ['source-node', 'alternate-node'],
    workflows: ['workflow-to-graft'],
    ...(sourceNodeId ? { sourceNodeId } : {}),
  };

  return mount(GraftNodesDialog as unknown as Component, {
    props,
    global: {
      stubs: {
        ...dialogStubs,
        ...selectStubs,
      },
    },
  });
}

function selectForTrigger(wrapper: ReturnType<typeof mount>, triggerSelector: string) {
  const trigger = wrapper.get(triggerSelector).element;
  const selects = wrapper.findAllComponents({ name: 'Select' });
  const select = selects.find((candidate) => (candidate.element as Element).contains(trigger));
  if (!select) {
    throw new Error(`No Select wraps ${triggerSelector}`);
  }
  return select;
}

async function makeWorkflowSubmittable(wrapper: ReturnType<typeof mount>) {
  await wrapper.get('#graft-workflow-id').setValue('workflow-to-graft');
  await flushPromises();
}

describe('GraftNodesDialog run-after default', () => {
  it('preselects originating node when opened from a node', () => {
    const wrapper = mountDialog('source-node');

    expect(selectForTrigger(wrapper, '#graft-connect-from').attributes('data-model-value'))
      .toBe('source-node');
  });

  it('allows override after preselect', async () => {
    const wrapper = mountDialog('source-node');
    const runAfter = selectForTrigger(wrapper, '#graft-connect-from');
    expect(runAfter.attributes('data-model-value')).toBe('source-node');

    runAfter.vm.$emit('update:modelValue', 'alternate-node');
    await makeWorkflowSubmittable(wrapper);
    await wrapper.get('form').trigger('submit');

    const submitted = wrapper.emitted('submit')?.[0];
    expect(submitted).toBeTruthy();
    expect(submitted?.[1]).toEqual([
      { source: 'alternate-node', target: expect.any(String), type: 'success' },
    ]);
  });

  it("falls back to 'no upstream edge' when no source node provided", () => {
    const wrapper = mountDialog();

    expect(selectForTrigger(wrapper, '#graft-connect-from').attributes('data-model-value'))
      .toBe(NO_UPSTREAM_EDGE);
  });
});
