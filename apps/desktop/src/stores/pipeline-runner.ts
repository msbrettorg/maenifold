// Polling, view-refresh, and view-sync helpers extracted from pipeline.ts
// to keep the main store under the 300 LOC ceiling.
import type { Ref } from 'vue';
import type { LogEntry, PipelineView } from '../lib/pipeline-types';
import type { usePipelineMcp } from '../composables/usePipelineMcp';
import { hydrateEventLogFromView } from '../composables/pipeline-view-diff';
import { normalizeView } from './pipeline-view-transforms';

const ACTIVE_POLLING_PIPELINE_STATUSES: ReadonlySet<PipelineView['status']> = new Set(['active']);
const TERMINAL_PIPELINE_STATUSES: ReadonlySet<PipelineView['status']> = new Set([
  'completed',
  'abandoned',
  'cancelled',
  'stuck',
]);

interface PipelineRunnerDeps {
  mcp: ReturnType<typeof usePipelineMcp>;
  currentPipelineId: Ref<string | null>;
  currentView: Ref<PipelineView | null>;
  eventLog: Ref<LogEntry[]>;
  setView: (view: PipelineView | null, metadata?: { templateId?: string }) => void;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T>;
}

export function createPipelineRunnerActions(deps: PipelineRunnerDeps) {
  const { mcp, currentPipelineId, currentView, eventLog, setView, withLoading } = deps;
  let stopPollingHandle: (() => void) | null = null;

  function stopCurrentPolling() {
    if (!stopPollingHandle) return;
    stopPollingHandle();
    stopPollingHandle = null;
  }

  function startViewPolling() {
    stopCurrentPolling();
    if (!currentPipelineId.value) return;
    stopPollingHandle = mcp.startPolling(currentPipelineId.value, (view) => {
      const nextView = normalizeView(view);
      if (currentView.value) {
        const entries = mcp.diffViews(currentView.value, nextView);
        if (entries.length > 0) {
          eventLog.value = [...eventLog.value, ...entries];
        }
      }
      setView(nextView);
    }, 3000);
  }

  function syncPollingState(view: PipelineView | null) {
    if (!view) {
      stopCurrentPolling();
      return;
    }

    const hasActiveNodes = Array.isArray(view.active) && view.active.length > 0;
    const hasRunningNodes =
      view.running === true || view.nodes.some((node) => node.status === 'active');

    if (
      ACTIVE_POLLING_PIPELINE_STATUSES.has(view.status)
      || hasActiveNodes
      || hasRunningNodes
    ) {
      if (!stopPollingHandle && currentPipelineId.value === view.pipelineId) {
        startViewPolling();
      }
      return;
    }

    if (TERMINAL_PIPELINE_STATUSES.has(view.status) || stopPollingHandle) {
      stopCurrentPolling();
    }
  }

  function resumePollingForActiveView() {
    syncPollingState(currentView.value);
  }

  async function refreshView(options?: { resetEventLog?: boolean }) {
    if (!currentPipelineId.value) return;
    await withLoading(async () => {
      const previousView = currentView.value;
      const nextView = normalizeView(await mcp.viewPipeline(currentPipelineId.value!));
      if (options?.resetEventLog) {
        eventLog.value = hydrateEventLogFromView(nextView);
      } else if (previousView) {
        const entries = mcp.diffViews(previousView, nextView);
        if (entries.length > 0) {
          eventLog.value = [...eventLog.value, ...entries];
        }
      }
      setView(nextView);
    });
  }

  return {
    startViewPolling,
    stopCurrentPolling,
    syncPollingState,
    resumePollingForActiveView,
    refreshView,
  };
}
