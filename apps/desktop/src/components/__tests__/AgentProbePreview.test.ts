// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { mount } from '@vue/test-utils';
import AgentProbePreview from '../AgentProbePreview.vue';
import { isOfflineProbeError, normalizeAgentProbeResult } from '../../lib/agent-probe-types';
import { useAgentCapabilitiesStore } from '../../stores/agent-capabilities';

describe('AgentProbePreview', () => {
  it('shows degraded cached messaging and exposes manual refresh', async () => {
    const pinia = createPinia();
    setActivePinia(pinia);

    const store = useAgentCapabilitiesStore();
    const refreshSpy = vi.spyOn(store, 'refreshAll').mockResolvedValue();
    const probe = {
      ...normalizeAgentProbeResult({ configOptions: [] }),
      degradedMessage: 'Live refresh timed out. Showing cached capabilities from 6m ago.',
    };

    const wrapper = mount(AgentProbePreview, {
      props: {
        agentName: 'architect',
        probe,
      },
      global: {
        plugins: [pinia],
      },
    });

    expect(wrapper.text()).toContain('Ready');
    expect(wrapper.text()).toContain('Live refresh timed out. Showing cached capabilities from 6m ago.');

    await wrapper.get('button.refresh-btn').trigger('click');

    expect(refreshSpy).toHaveBeenCalledTimes(1);
  });

  it('shows a non-blocking warning when discovery falls back to configured agents', () => {
    const pinia = createPinia();
    setActivePinia(pinia);

    const store = useAgentCapabilitiesStore();
    store.warningMessage =
      'Live agent capability probe timed out. Showing configured agents and any cached capabilities until refresh succeeds.';

    const wrapper = mount(AgentProbePreview, {
      props: {
        agentName: 'architect',
      },
      global: {
        plugins: [pinia],
      },
    });

    expect(wrapper.text()).toContain('Pending');
    expect(wrapper.text()).toContain('Live capability discovery is unavailable for architect. Use Refresh to retry loading dynamic options.');
    expect(wrapper.text()).toContain('Live agent capability probe timed out.');
  });

  it('keeps offline no-cache state non-blocking for configured agents', () => {
    const pinia = createPinia();
    setActivePinia(pinia);

    const wrapper = mount(AgentProbePreview, {
      props: {
        agentName: 'architect',
        probe: normalizeAgentProbeResult({
          error: 'MCP client is not connected',
        }),
      },
      global: {
        plugins: [pinia],
      },
    });

    expect(isOfflineProbeError('MCP client is not connected')).toBe(true);
    expect(wrapper.text()).toContain('Offline');
    expect(wrapper.text()).toContain('Live capability discovery is unavailable for architect. Cached options stay visible when present, and Refresh retries MCP discovery.');
  });

  it('maps unexpected capability payload errors to an actionable degraded state', () => {
    const pinia = createPinia();
    setActivePinia(pinia);

    const wrapper = mount(AgentProbePreview, {
      props: {
        agentName: 'Copilot',
        probe: normalizeAgentProbeResult({
          error: 'Agent returned an unexpected capability payload',
        }),
      },
      global: {
        plugins: [pinia],
      },
    });

    expect(wrapper.text()).toContain('Needs attention');
    expect(wrapper.text()).toContain('Capability metadata for Copilot could not be interpreted.');
    expect(wrapper.text()).toContain('Verify the agent command in Settings > Agents, then Refresh.');
    expect(wrapper.text()).toContain('Agent returned an unexpected capability payload');
    expect(wrapper.text()).not.toContain('Probe failed');
  });
});
