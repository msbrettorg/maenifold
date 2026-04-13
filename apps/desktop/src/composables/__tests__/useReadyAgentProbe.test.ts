import { computed, nextTick, ref } from 'vue';
import { describe, expect, it, vi } from 'vitest';
import { normalizeAgentProbeResult } from '../../lib/agent-probe-types';
import type { PipelineNodeAgentConfig } from '../../lib/pipeline-types';
import { useReadyAgentProbe } from '../useReadyAgentProbe';

describe('useReadyAgentProbe', () => {
  it('seeds defaults once when the ready probe advertises concrete defaults', async () => {
    const probe = ref(
      normalizeAgentProbeResult({
        configOptions: [
          {
            id: 'mode',
            name: 'Mode',
            category: 'mode',
            currentValue: 'prompt',
            options: [{ value: 'prompt', name: 'Prompt' }],
          },
        ],
      }),
    );
    const config = ref<PipelineNodeAgentConfig | undefined>(undefined);
    const applyDefaults = vi.fn((nextConfig: PipelineNodeAgentConfig) => {
      config.value = nextConfig;
    });

    const readyProbe = useReadyAgentProbe(
      computed(() => probe.value),
      computed(() => config.value),
      applyDefaults,
    );

    await nextTick();

    expect(readyProbe.value?.state).toBe('ready');
    expect(applyDefaults).toHaveBeenCalledTimes(1);
    expect(config.value).toEqual({ mode: 'prompt' });
  });

  it('does not try to seed defaults when the probe has no concrete values', async () => {
    const probe = ref(
      normalizeAgentProbeResult({
        configOptions: [
          {
            id: 'model',
            name: 'Model',
            category: 'model',
            options: [],
          },
        ],
      }),
    );
    const config = ref<PipelineNodeAgentConfig | undefined>(undefined);
    const applyDefaults = vi.fn();

    const readyProbe = useReadyAgentProbe(
      computed(() => probe.value),
      computed(() => config.value),
      applyDefaults,
    );

    await nextTick();

    expect(readyProbe.value?.state).toBe('ready');
    expect(applyDefaults).not.toHaveBeenCalled();
  });

  it('does not overwrite an explicit agent config', async () => {
    const probe = ref(
      normalizeAgentProbeResult({
        configOptions: [
          {
            id: 'mode',
            name: 'Mode',
            category: 'mode',
            currentValue: 'workflow',
            options: [{ value: 'workflow', name: 'Workflow' }],
          },
        ],
      }),
    );
    const config = ref<PipelineNodeAgentConfig | undefined>({ mode: 'prompt' });
    const applyDefaults = vi.fn();

    useReadyAgentProbe(
      computed(() => probe.value),
      computed(() => config.value),
      applyDefaults,
    );

    await nextTick();

    expect(applyDefaults).not.toHaveBeenCalled();
  });
});
