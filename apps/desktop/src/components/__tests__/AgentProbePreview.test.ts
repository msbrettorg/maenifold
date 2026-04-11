// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { mount } from '@vue/test-utils';
import AgentProbePreview from '../AgentProbePreview.vue';
import { normalizeAgentProbeResult } from '../../lib/agent-probe-types';
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
});
