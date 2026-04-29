// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import WorkflowParameterDialog from '../WorkflowParameterDialog.vue';

const dialogStubs = {
  Dialog: { template: '<div><slot /></div>', props: ['open'] },
  DialogContent: { template: '<div><slot /></div>' },
  DialogDescription: { template: '<p><slot /></p>' },
  DialogFooter: { template: '<div><slot /></div>' },
  DialogHeader: { template: '<div><slot /></div>' },
  DialogTitle: { template: '<h2><slot /></h2>' },
  Input: {
    template: '<input :name="name" :value="modelValue" @input="$emit(\'update:modelValue\', $event.target.value)" />',
    props: ['modelValue', 'name'],
    emits: ['update:modelValue'],
  },
  Select: { template: '<div><slot /></div>', props: ['modelValue'] },
  SelectContent: { template: '<div><slot /></div>' },
  SelectItem: { template: '<button type="button"><slot /></button>', props: ['value'] },
  SelectTrigger: { template: '<button type="button"><slot /></button>' },
  SelectValue: { template: '<span><slot /></span>', props: ['placeholder'] },
};

describe('WorkflowParameterDialog', () => {
  it('surfaces invalid number coercion as validation text', async () => {
    const submit = vi.fn();
    const wrapper = mount(WorkflowParameterDialog, {
      props: {
        open: true,
        inputs: {
          amount: { label: 'Amount', type: 'number' },
        },
        error: null,
        submitting: false,
        onSubmit: submit,
      },
      global: { stubs: dialogStubs },
    });

    const vm = wrapper.vm as unknown as { values: Record<string, string> };
    vm.values.amount = '3.14abc';
    await wrapper.get('[data-testid="workflow-parameter-submit"]').trigger('click');

    expect(wrapper.text()).toContain('Invalid number for "amount": 3.14abc');
    expect(submit).not.toHaveBeenCalled();
  });
});
