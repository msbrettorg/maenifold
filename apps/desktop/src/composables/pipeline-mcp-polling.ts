import { useIntervalFn } from '@vueuse/core';
import type { Ref } from 'vue';
import type { PipelineView } from '../lib/pipeline-types-view';

interface PipelinePollingOptions {
  pollingError: Ref<string | null>;
  reconnecting: Ref<boolean>;
  viewPipeline: (sessionId: string) => Promise<PipelineView>;
}

export function createPipelinePolling({ pollingError, reconnecting, viewPipeline }: PipelinePollingOptions) {
  return function startPolling(
    sessionId: string,
    onUpdate: (view: PipelineView) => void,
    intervalMs = 3000,
  ): () => void {
    let stopped = false;
    let consecutiveFailures = 0;
    pollingError.value = null;

    const { pause } = useIntervalFn(async () => {
      if (stopped) return;

      try {
        const view = await viewPipeline(sessionId);
        consecutiveFailures = 0;
        pollingError.value = null;
        onUpdate(view);
      } catch (error) {
        if (reconnecting.value) {
          return;
        }

        consecutiveFailures += 1;
        if (consecutiveFailures < 3) {
          return;
        }

        pollingError.value = `Polling stopped after 3 consecutive failures: ${
          error instanceof Error ? error.message : String(error)
        }`;
        stopped = true;
        pause();
      }
    }, intervalMs, { immediate: true });

    return () => {
      stopped = true;
      pause();
    };
  };
}
