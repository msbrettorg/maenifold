/* @vitest-environment jsdom */

import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';

import SettingsSchemaField from './SettingsSchemaField.vue';
import type { JsonSchemaNode } from '../lib/settingsSchema';

function mountNumberField(value: unknown) {
  const schema: JsonSchemaNode = {
    type: 'number',
    title: 'Debounce ms',
  };

  return mount(SettingsSchemaField, {
    props: {
      path: ['debounceMs'],
      fieldKey: 'debounceMs',
      schema,
      value,
    },
  });
}

describe('SettingsSchemaField', () => {
  it('hides number validation for pristine unset fields', () => {
    const wrapper = mountNumberField(undefined);

    expect(wrapper.find('.field-error').exists()).toBe(false);
    expect(wrapper.find('button.primary').attributes('disabled')).toBeDefined();
  });

  it('shows number validation after clearing a previously valid value', async () => {
    const wrapper = mountNumberField(42);
    const input = wrapper.get('input[type="number"]');

    await input.setValue('');

    expect(wrapper.get('.field-error').text()).toBe('Enter a number before saving.');
    expect(wrapper.find('button.primary').attributes('disabled')).toBeDefined();
  });
});
