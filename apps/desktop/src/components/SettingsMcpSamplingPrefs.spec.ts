// @vitest-environment jsdom
import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import SettingsMcpSamplingPrefs from './SettingsMcpSamplingPrefs.vue';
import type { AgentsConfig } from '../lib/types';
import { useConfigStore } from '../stores/config';

const stubs = {
  Badge: { template: '<span><slot /></span>' },
  Card: { template: '<article v-bind="$attrs"><slot /></article>' },
  CardContent: { template: '<div><slot /></div>' },
  CardDescription: { template: '<p><slot /></p>' },
  CardHeader: { template: '<header><slot /></header>' },
  CardTitle: { template: '<h3><slot /></h3>' },
  Input: {
    props: ['modelValue', 'disabled'],
    emits: ['update:modelValue', 'change'],
    template:
      '<input v-bind="$attrs" :disabled="disabled" :value="modelValue" @input="$emit(\'update:modelValue\', $event.target.value)" @change="$emit(\'change\', $event)" />',
  },
  Switch: {
    props: ['checked', 'disabled', 'ariaLabel'],
    emits: ['update:checked'],
    template:
      '<button type="button" role="switch" :aria-label="ariaLabel" :aria-checked="String(checked)" :disabled="disabled" @click="$emit(\'update:checked\', !checked)" />',
  },
};

function setupConfig(config: AgentsConfig) {
  setActivePinia(createPinia());
  const configStore = useConfigStore();
  configStore.config = config;
  const setSamplingPreference = vi.fn(async () => undefined);
  const setDefaultToolLoopBudget = vi.fn(async () => undefined);
  Object.assign(configStore, { setSamplingPreference, setDefaultToolLoopBudget });
  return { setSamplingPreference, setDefaultToolLoopBudget };
}

function mountSubject() {
  return mount(SettingsMcpSamplingPrefs, { global: { stubs } });
}

describe('SettingsMcpSamplingPrefs tool loop budget inputs', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders per-server tool loop input with the configured server value', () => {
    setupConfig({
      agents: {},
      mcpServers: [{ type: 'stdio', name: 'foo', command: 'npx' }],
      samplingPreferences: {
        defaultToolLoopBudget: 10,
        perServer: { foo: { alwaysAllow: true, toolLoopBudget: 7 } },
      },
    });

    const wrapper = mountSubject();

    expect((wrapper.get('#tool-loop-budget-foo').element as HTMLInputElement).value).toBe('7');
    wrapper.unmount();
  });

  it('changing per-server input dispatches setSamplingPreference with toolLoopBudget', async () => {
    const { setSamplingPreference } = setupConfig({
      agents: {},
      mcpServers: [{ type: 'stdio', name: 'foo', command: 'npx' }],
      samplingPreferences: { perServer: { foo: { alwaysAllow: true, modelOverride: 'gpt-5.4' } } },
    });
    const wrapper = mountSubject();

    await wrapper.get('#tool-loop-budget-foo').setValue('12');
    await flushPromises();

    expect(setSamplingPreference).toHaveBeenCalledWith('foo', {
      alwaysAllow: true,
      modelOverride: 'gpt-5.4',
      toolLoopBudget: 12,
    });
    wrapper.unmount();
  });

  it('renders default budget input with fallback value 10', () => {
    setupConfig({
      agents: {},
      mcpServers: [{ type: 'stdio', name: 'foo', command: 'npx' }],
      samplingPreferences: { perServer: { foo: { alwaysAllow: false } } },
    });

    const wrapper = mountSubject();

    expect((wrapper.get('#default-tool-loop-budget').element as HTMLInputElement).value).toBe('10');
    wrapper.unmount();
  });
});
