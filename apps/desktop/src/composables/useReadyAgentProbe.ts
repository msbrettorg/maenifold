import { computed, watch, type ComputedRef } from 'vue';
import type { AgentProbeResult } from '../lib/agent-probe-types';
import type { PipelineNodeAgentConfig } from '../lib/pipeline-types';
import { buildAgentConfigDefaults } from './pipeline-node-form-helpers';

function hasExplicitAgentConfig(
  value: PipelineNodeAgentConfig | undefined,
): boolean {
  if (!value) {
    return false;
  }

  return Object.keys(value).length > 0;
}

export function useReadyAgentProbe(
  probe: ComputedRef<AgentProbeResult | undefined>,
  config: ComputedRef<PipelineNodeAgentConfig | undefined>,
  applyDefaults: (value: PipelineNodeAgentConfig) => void,
) {
  watch(
    [probe, config],
    ([nextProbe, nextConfig]) => {
      if (nextProbe?.state !== 'ready' || hasExplicitAgentConfig(nextConfig)) {
        return;
      }

      const defaults = buildAgentConfigDefaults(nextProbe);
      if (defaults) {
        applyDefaults(defaults);
      }
    },
    { immediate: true },
  );

  return computed(() =>
    probe.value?.state === 'ready' ? probe.value : undefined,
  );
}
