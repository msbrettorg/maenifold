// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import type { Component } from 'vue';

import PipelineInputSchemaForm from '../PipelineInputSchemaForm.vue';

const schema = {
  type: 'object',
  required: ['project_name'],
  properties: {
    project_name: { type: 'string' },
    count: { type: 'integer' },
    active: { type: 'boolean' },
    mode: { type: 'string', enum: ['a', 'b'] },
  },
};

const selectStubs = {
  Select: {
    name: 'Select',
    props: ['modelValue', 'name'],
    emits: ['update:modelValue'],
    template:
      '<div data-testid="select-stub" :data-name="name" :data-model-value="modelValue"><slot /></div>',
  },
  SelectContent: { template: '<div><slot /></div>' },
  SelectItem: {
    props: ['value'],
    template: '<div role="option" :data-value="value"><slot /></div>',
  },
  SelectTrigger: { template: '<button type="button" v-bind="$attrs"><slot /></button>' },
  SelectValue: { props: ['placeholder'], template: '<span>{{ placeholder }}</span>' },
};

const checkboxStub = {
  Checkbox: {
    name: 'Checkbox',
    props: ['checked', 'name', 'id'],
    emits: ['update:checked'],
    template:
      '<input type="checkbox" :id="id" :name="name" :checked="checked" @change="$emit(\'update:checked\', $event.target.checked)" />',
  },
};

function mountForm(modelValue = {}) {
  return mount(PipelineInputSchemaForm as unknown as Component, {
    props: {
      schema,
      modelValue,
      onSubmit: vi.fn(),
      'onUpdate:modelValue': vi.fn(),
    },
    global: {
      stubs: {
        ...selectStubs,
        ...checkboxStub,
      },
    },
  });
}

describe('PipelineInputSchemaForm', () => {
  it('renders typed controls from schema properties', () => {
    const wrapper = mountForm();

    expect(wrapper.find('input[name="project_name"][type="text"]').exists()).toBe(true);
    expect(wrapper.find('input[name="count"][type="number"]').exists()).toBe(true);
    expect(wrapper.find('input[name="active"][type="checkbox"]').exists()).toBe(true);

    const select = wrapper.find('[data-testid="select-stub"][data-name="mode"]');
    expect(select.exists()).toBe(true);
    expect(select.findAll('[role="option"]').map((option) => option.text())).toEqual(['a', 'b']);
  });

  it('blocks submit when a required string is empty', async () => {
    const wrapper = mountForm({ project_name: '', count: 1, active: false, mode: 'a' });

    await wrapper.get('form').trigger('submit.prevent');

    expect(wrapper.emitted('submit')).toBeUndefined();
    expect(wrapper.text()).toContain('project_name');
  });

  it('emits typed object when a valid draft is submitted', async () => {
    const wrapper = mountForm({ active: false });

    await wrapper.get('input[name="project_name"]').setValue('demo');
    await wrapper.get('input[name="count"]').setValue('7');

    const checkbox = wrapper.get('input[name="active"][type="checkbox"]')
      .element as HTMLInputElement;
    checkbox.checked = true;
    await wrapper.get('input[name="active"][type="checkbox"]').trigger('change');

    const modeSelect = wrapper.findComponent({ name: 'Select' });
    await modeSelect.vm.$emit('update:modelValue', 'b');

    await wrapper.get('form').trigger('submit.prevent');

    const payload =
      wrapper.emitted('submit')?.at(0)?.at(0) ??
      wrapper.emitted('update:modelValue')?.at(-1)?.at(0);

    expect(payload).toEqual({
      project_name: 'demo',
      count: 7,
      active: true,
      mode: 'b',
    });
  });
});
