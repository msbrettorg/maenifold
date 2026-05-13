// @vitest-environment jsdom
import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import SettingsMcpSamplingPrefs from '../SettingsMcpSamplingPrefs.vue';
import { useConfigStore } from '../../stores/config';
import type { AgentsConfig } from '../../lib/types';

const stubs = {
  Badge: { template: '<span><slot /></span>' },
  Card: { template: '<article v-bind="$attrs"><slot /></article>' },
  CardContent: { template: '<div><slot /></div>' },
  CardDescription: { template: '<p><slot /></p>' },
  CardHeader: { template: '<header><slot /></header>' },
  CardTitle: { template: '<h3><slot /></h3>' },
  Switch: {
    props: ['checked', 'disabled', 'ariaLabel'],
    emits: ['update:checked'],
    template:
      '<button type="button" role="switch" :aria-label="ariaLabel" :aria-checked="String(checked)" :data-state="checked ? \'checked\' : \'unchecked\'" :disabled="disabled" @click="$emit(\'update:checked\', !checked)" />',
  },
};

function mountSubject() {
  return mount(SettingsMcpSamplingPrefs, { global: { stubs } });
}

function setupConfig(config: AgentsConfig) {
  setActivePinia(createPinia());
  const configStore = useConfigStore();
  configStore.config = config;
  const setSamplingPreference = vi.fn(async () => undefined);
  const writeConfig = vi.fn(async () => undefined);
  Object.assign(configStore, { setSamplingPreference, writeConfig });
  return { configStore, setSamplingPreference, writeConfig };
}

function switchFor(wrapper: ReturnType<typeof mountSubject>, serverName: string) {
  return wrapper.get(`[role="switch"][aria-label="Always allow sampling for ${serverName}"]`);
}

describe('SettingsMcpSamplingPrefs', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders empty state when no MCP servers configured', () => {
    setupConfig({ agents: {}, mcpServers: [] });
    const wrapper = mountSubject();

    expect(wrapper.text()).toContain('No MCP servers configured.');
  });

  it('renders one card per configured MCP server', () => {
    setupConfig({
      agents: {},
      mcpServers: [
        { type: 'stdio', name: 'a', command: 'npx' },
        { type: 'http', name: 'b', url: 'https://example.com/mcp' },
      ],
    });
    const wrapper = mountSubject();

    expect(wrapper.findAll('[data-testid="mcp-sampling-pref-card"]')).toHaveLength(2);
  });

  it('toggle bound to samplingPreferences.perServer[name].alwaysAllow', () => {
    setupConfig({
      agents: {},
      mcpServers: [{ type: 'stdio', name: 'a', command: 'npx' }],
      samplingPreferences: { perServer: { a: { alwaysAllow: true } } },
    });
    const wrapper = mountSubject();

    expect(switchFor(wrapper, 'a').attributes('aria-checked')).toBe('true');
    expect(switchFor(wrapper, 'a').attributes('data-state')).toBe('checked');
  });

  it('toggle change calls setSamplingPreference helper', async () => {
    const { setSamplingPreference, writeConfig } = setupConfig({
      agents: {},
      mcpServers: [{ type: 'stdio', name: 'server-name', command: 'npx' }],
    });
    const wrapper = mountSubject();

    await switchFor(wrapper, 'server-name').trigger('click');
    await flushPromises();

    expect(setSamplingPreference).toHaveBeenCalledWith('server-name', { alwaysAllow: true });
    expect(writeConfig).not.toHaveBeenCalled();
  });

  it('toggle preserves prefs for OTHER servers', async () => {
    const { setSamplingPreference, writeConfig } = setupConfig({
      agents: {},
      mcpServers: [
        { type: 'stdio', name: 'a', command: 'npx' },
        { type: 'http', name: 'b', url: 'https://example.com/mcp' },
      ],
      samplingPreferences: {
        perServer: {
          a: { alwaysAllow: false },
          b: { alwaysAllow: true, modelOverride: 'gpt-4o' },
        },
      },
    });
    const wrapper = mountSubject();

    await switchFor(wrapper, 'a').trigger('click');
    await flushPromises();

    expect(setSamplingPreference).toHaveBeenCalledTimes(1);
    expect(setSamplingPreference).toHaveBeenCalledWith('a', { alwaysAllow: true });
    expect(setSamplingPreference).not.toHaveBeenCalledWith('b', expect.anything());
    expect(writeConfig).not.toHaveBeenCalled();
  });

  it('error from helper surfaces inline', async () => {
    const { setSamplingPreference } = setupConfig({
      agents: {},
      mcpServers: [{ type: 'stdio', name: 'a', command: 'npx' }],
    });
    setSamplingPreference.mockRejectedValueOnce(new Error('save failed'));
    const wrapper = mountSubject();

    await switchFor(wrapper, 'a').trigger('click');
    await flushPromises();

    expect(wrapper.text()).toContain('save failed');
    expect(wrapper.get('[role="alert"]').text()).toContain('save failed');
  });

  it('transport badge renders for stdio vs http', () => {
    setupConfig({
      agents: {},
      mcpServers: [
        { type: 'stdio', name: 'local', command: 'npx' },
        { type: 'http', name: 'remote', url: 'https://example.com/mcp' },
      ],
    });
    const wrapper = mountSubject();

    expect(wrapper.text()).toContain('Stdio');
    expect(wrapper.text()).toContain('Http');
  });
});
