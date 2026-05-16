// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import SessionConfigPanel from '../SessionConfigPanel.vue';
import { normalizeAgentProbeResult } from '../../lib/agent-probe-normalizers';
import { useAgentCapabilitiesStore } from '../../stores/agent-capabilities';
import { useAgentSessionOverridesStore } from '../../stores/agent-session-overrides';

const agentName = 'Architect';
const modelConfigId = 'model';

const selectStubs = {
  Select: {
    name: 'Select',
    props: ['modelValue', 'disabled'],
    emits: ['update:modelValue'],
    template: '<div data-test="model-select" :data-model-value="modelValue" :data-disabled="String(disabled)"><slot /></div>',
  },
  SelectTrigger: { name: 'SelectTrigger', template: '<button type="button"><slot /></button>' },
  SelectContent: { name: 'SelectContent', template: '<div><slot /></div>' },
  SelectItem: { name: 'SelectItem', props: ['value'], template: '<div :data-value="value"><slot /></div>' },
  SelectValue: { name: 'SelectValue', props: ['placeholder'], template: '<span>{{ placeholder }}</span>' },
  Textarea: { name: 'Textarea', template: '<textarea />' },
};

function mountPanel() {
  const pinia = createPinia();
  setActivePinia(pinia);

  const capStore = useAgentCapabilitiesStore();
  capStore.probeResults = new Map([
    [
      agentName,
      normalizeAgentProbeResult({
        configOptions: {
          [modelConfigId]: {
            id: modelConfigId,
            name: 'Model',
            category: 'model',
            type: 'select',
            currentValue: '',
            options: [
              { value: 'gpt-5.4', name: 'GPT 5.4' },
              { value: 'claude-sonnet-4.6', name: 'Claude Sonnet 4.6' },
            ],
          },
        },
      }),
    ],
  ]);

  const overridesStore = useAgentSessionOverridesStore();
  const setSelectedOverride = vi.spyOn(overridesStore, 'setSelectedOverride');
  const wrapper = mount(SessionConfigPanel, {
    props: {
      agentName,
      cwd: '/repo',
      isReadOnly: false,
    },
    global: {
      plugins: [pinia],
      stubs: selectStubs,
    },
  });

  return { overridesStore, setSelectedOverride, wrapper };
}

describe('SessionConfigPanel', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('clears the model override when the Select emits null', async () => {
    const { overridesStore, setSelectedOverride, wrapper } = mountPanel();

    wrapper.getComponent({ name: 'Select' }).vm.$emit('update:modelValue', null);
    await wrapper.vm.$nextTick();

    expect(setSelectedOverride).toHaveBeenCalledWith(agentName, modelConfigId, '');
    expect(overridesStore.getSelectedOverride(agentName, modelConfigId)).toBe('');
  });

  it('clears the model override when the Select emits undefined', async () => {
    const { overridesStore, setSelectedOverride, wrapper } = mountPanel();

    wrapper.getComponent({ name: 'Select' }).vm.$emit('update:modelValue', undefined);
    await wrapper.vm.$nextTick();

    expect(setSelectedOverride).toHaveBeenCalledWith(agentName, modelConfigId, '');
    expect(overridesStore.getSelectedOverride(agentName, modelConfigId)).toBe('');
  });

  it('leaves the override store untouched for non-string model values', async () => {
    const { overridesStore, setSelectedOverride, wrapper } = mountPanel();
    overridesStore.setSelectedOverride(agentName, modelConfigId, 'gpt-5.4');
    setSelectedOverride.mockClear();

    wrapper.getComponent({ name: 'Select' }).vm.$emit('update:modelValue', 42);
    await wrapper.vm.$nextTick();

    expect(setSelectedOverride).not.toHaveBeenCalled();
    expect(overridesStore.getSelectedOverride(agentName, modelConfigId)).toBe('gpt-5.4');
  });
});
