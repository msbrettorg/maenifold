// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import ModePicker from '../ModePicker.vue';
import type { SessionMode } from '../../lib/types';

const modes: SessionMode[] = [
  { id: 'plan', name: 'Plan', description: 'Read-only planning', _meta: null },
  { id: 'act', name: 'Act', description: 'Execute changes', _meta: null },
];

describe('ModePicker', () => {
  it('renders nothing when modes is empty', () => {
    const wrapper = mount(ModePicker, {
      props: { modes: [], currentModeId: null },
    });

    expect(wrapper.find('[data-testid="mode-picker"]').exists()).toBe(false);
  });

  it('renders one pill per mode', () => {
    const wrapper = mount(ModePicker, {
      props: { modes, currentModeId: 'plan' },
    });

    expect(wrapper.findAll('[data-slot="button"]')).toHaveLength(2);
    expect(wrapper.text()).toContain('Plan');
    expect(wrapper.text()).toContain('Act');
  });

  it('marks the current mode pill with the accent class', () => {
    const wrapper = mount(ModePicker, {
      props: { modes, currentModeId: 'act' },
    });

    expect(wrapper.get('[data-testid="mode-picker-pill-act"]').classes()).toContain('bg-accent');
    expect(wrapper.get('[data-testid="mode-picker-pill-plan"]').classes()).not.toContain('bg-accent');
  });

  it('emits select-mode with the chosen mode id', async () => {
    const wrapper = mount(ModePicker, {
      props: { modes, currentModeId: 'plan' },
    });

    await wrapper.get('[data-testid="mode-picker-pill-act"]').trigger('click');

    expect(wrapper.emitted('select-mode')).toEqual([[{ modeId: 'act' }]]);
  });
});
