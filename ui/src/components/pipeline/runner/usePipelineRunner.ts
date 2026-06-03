// Pipeline runner composable — the live orchestration seam (T-600.3).
//
// Owns the 3s poll loop and wraps the store's lifecycle ops so the runner
// components stay declarative. Pure decision logic lives in the sibling `.ts`
// modules (runnerLog/runnerForm/runnerHistory/runnerActions); this file only
// wires those decisions to the interval, the store, and component lifecycle.
//
// Polling contract:
//  - poll `store.fetchViewRaw(selectedId)` (non-mutating) every POLL_INTERVAL_MS
//    while the pipeline is non-terminal, applying the result only when the poll
//    is still current (generation/dispose/id gating — see runnerActions.isPollCurrent);
//  - diff successive views into LogEntry[] and append to the store event log;
//  - tolerate up to MAX_POLL_STRIKES consecutive errors before surfacing/stopping;
//  - stop on terminal status, on pipeline switch, and on unmount.

import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { usePipelinesStore } from '../../../stores/pipelines';
import type {
  PipelineDefinition,
  PipelineInputs,
  PipelineView,
} from '../../../lib/pipeline-types';
import { diffViews, isTerminalStatus, type LogEntry } from './runnerLog';
import { actionFlags, isPollCurrent, shouldCancelRemote } from './runnerActions';

const POLL_INTERVAL_MS = 3000;
const MAX_POLL_STRIKES = 3;

function toMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export interface CreateOptions {
  name: string;
  description?: string;
  maxConcurrency?: number;
}

export function usePipelineRunner() {
  const store = usePipelinesStore();
  const { selectedView, selectedPipelineId, eventLog, pollingActive, pollError } =
    storeToRefs(store);

  // Interval handle + diff baseline + strike counter are local, non-reactive
  // orchestration state. `lastView` is the previous snapshot the next poll diffs
  // against; null means "next poll is the first of this watch session".
  let timer: ReturnType<typeof setInterval> | null = null;
  let lastView: PipelineView | null = null;
  let strikes = 0;
  // Monotonic generation token: bumped on every (re)start and stop of polling.
  // A poll captures it at START and bails on resolve if it changed (the user
  // switched pipelines, or the loop was torn down) — see runnerActions.isPollCurrent.
  let generation = 0;
  let disposed = false;

  // Surfaced so components can disable buttons / show a force-delete confirm.
  const busy = ref(false);
  const actionError = ref<string | null>(null);
  const deleteNeedsForce = ref(false);

  const flags = computed(() => actionFlags(selectedView.value));

  function clearTimer(): void {
    if (timer != null) {
      clearInterval(timer);
      timer = null;
    }
  }

  function stopPolling(): void {
    clearTimer();
    // Bump the generation so any in-flight poll's result is discarded on resolve.
    generation += 1;
    store.setPollingActive(false);
  }

  function logEntry(level: LogEntry['level'], message: string): LogEntry {
    return { id: `${Date.now()}-local`, at: Date.now(), level, nodeId: null, message };
  }

  async function pollOnce(): Promise<void> {
    const capturedId = selectedPipelineId.value;
    const capturedGeneration = generation;
    if (!capturedId) {
      stopPolling();
      return;
    }
    try {
      // NON-mutating read: never assigns selectedView/selectedPipelineId.
      const view = await store.fetchViewRaw(capturedId);
      // Gate: discard if disposed, the generation changed (switch/restart/stop),
      // or the live selection moved off the polled id. This is what prevents an
      // in-flight poll for A from clobbering B's selection and log after a switch.
      if (
        !isPollCurrent({
          capturedId,
          capturedGeneration,
          currentId: selectedPipelineId.value,
          currentGeneration: generation,
          disposed,
        })
      ) {
        return;
      }
      // Still current: apply the view ourselves (keeps canvas/inspector live) and
      // diff/append the event log.
      store.applyPolledView(view);
      const entries = diffViews(lastView, view, Date.now());
      store.appendLogEntries(entries);
      lastView = view;
      strikes = 0;
      store.setPollError(null);
      if (isTerminalStatus(view.status)) {
        stopPolling();
      }
    } catch (error) {
      // Ignore errors from a poll that is no longer current.
      if (disposed || capturedGeneration !== generation || selectedPipelineId.value !== capturedId) {
        return;
      }
      strikes += 1;
      const message = toMessage(error);
      store.setPollError(message);
      if (strikes >= MAX_POLL_STRIKES) {
        store.appendLogEntries([
          logEntry('error', `Polling stopped after ${MAX_POLL_STRIKES} consecutive errors: ${message}`),
        ]);
        stopPolling();
      }
    }
  }

  // Begin (or restart) the poll loop. No-op for a terminal/absent pipeline.
  // `lastView` starts null so the first tick emits the "watching" entry.
  function startPolling(): void {
    clearTimer();
    const view = selectedView.value;
    if (disposed || !view || isTerminalStatus(view.status)) {
      store.setPollingActive(false);
      return;
    }
    // New generation so any still-pending poll from a prior loop is discarded.
    generation += 1;
    lastView = null;
    strikes = 0;
    store.setPollError(null);
    store.setPollingActive(true);
    timer = setInterval(() => {
      void pollOnce();
    }, POLL_INTERVAL_MS);
    // Kick an immediate tick so the operator sees state without a 3s wait.
    void pollOnce();
  }

  async function withBusy<T>(run: () => Promise<T>): Promise<T | undefined> {
    busy.value = true;
    actionError.value = null;
    try {
      return await run();
    } catch (error) {
      actionError.value = toMessage(error);
      return undefined;
    } finally {
      busy.value = false;
    }
  }

  // --- lifecycle actions ----------------------------------------------------

  async function run(options: { prompt?: string; inputs?: PipelineInputs } = {}): Promise<void> {
    const id = selectedPipelineId.value;
    if (!id) return;
    await withBusy(async () => {
      await store.runPipeline(id, options);
      store.resetEventLog();
      startPolling();
    });
  }

  async function pause(): Promise<void> {
    const id = selectedPipelineId.value;
    if (!id) return;
    await withBusy(async () => {
      await store.pausePipeline(id);
      stopPolling();
      await store.viewPipeline(id);
    });
  }

  async function resume(): Promise<void> {
    const id = selectedPipelineId.value;
    if (!id) return;
    await withBusy(async () => {
      await store.resumePipeline(id);
      lastView = selectedView.value;
      startPolling();
    });
  }

  // Cancel split: backend cancel only when the pipeline is remote-running;
  // otherwise local cleanup (stop polling, note it) with no backend round-trip.
  async function cancel(): Promise<void> {
    const id = selectedPipelineId.value;
    const view = selectedView.value;
    if (!id) return;
    if (!shouldCancelRemote(view)) {
      stopPolling();
      store.appendLogEntries([logEntry('warn', `Cancelled locally (no remote run to stop).`)]);
      return;
    }
    await withBusy(async () => {
      stopPolling();
      await store.cancelPipeline(id);
      await store.refreshList();
      await store.viewPipeline(id);
      store.appendLogEntries([logEntry('warn', `Cancel requested for ${id}.`)]);
    });
  }

  // Delete with force-recovery: a plain delete that errors (completed/unreadable
  // pipelines require force) flips `deleteNeedsForce` so the UI can confirm and
  // retry with force=true. We never auto-force and never silent-fail.
  async function remove(force = false): Promise<boolean> {
    const id = selectedPipelineId.value;
    if (!id) return false;
    busy.value = true;
    actionError.value = null;
    try {
      stopPolling();
      await store.deletePipeline(id, force);
      deleteNeedsForce.value = false;
      // Clear the now-deleted selection (and its log) so the UI never targets a
      // gone pipeline; the selection watcher tears down any remaining loop.
      store.clearSelection();
      await store.refreshList();
      return true;
    } catch (error) {
      actionError.value = toMessage(error);
      // Offer the force path only on the first (non-force) attempt's failure.
      if (!force) deleteNeedsForce.value = true;
      return false;
    } finally {
      busy.value = false;
    }
  }

  function dismissForceDelete(): void {
    deleteNeedsForce.value = false;
  }

  // Minimal create entry. The full node/edge designer is T-600.4; this seeds an
  // empty-graph pipeline carrying name/description/maxConcurrency so the operator
  // has a created session to design into. See WorkResult confession.
  async function create(options: CreateOptions): Promise<void> {
    const definition: PipelineDefinition = { nodes: [], edges: [] };
    await withBusy(async () => {
      await store.createPipeline(definition, {
        name: options.name,
        description: options.description,
        maxConcurrency: options.maxConcurrency,
      });
      await store.refreshList();
    });
  }

  // Reset runner state for the current selection and auto-start watching when
  // the selected pipeline is present and non-terminal. Shared by the selection
  // watcher and the on-mount path (navigating back to the tab with a pipeline
  // already selected). `startPolling` itself guards terminal/absent/disposed and
  // bumps the generation, so this can't double-start or fire for terminal ones.
  function syncToSelection(): void {
    stopPolling();
    store.resetEventLog();
    deleteNeedsForce.value = false;
    actionError.value = null;
    lastView = null;
    const view = selectedView.value;
    if (view && !isTerminalStatus(view.status)) {
      startPolling();
    }
  }

  // On pipeline switch: tear down the old loop, reset state, and (re)start.
  // `viewPipeline` sets `selectedView` before `selectedPipelineId`, so the new
  // view is already present when this fires.
  watch(selectedPipelineId, () => {
    syncToSelection();
  });

  // Mount with an already-selected pipeline (e.g. returning to the Pipelines
  // tab): the watcher is not `immediate`, so kick the same logic once here.
  onMounted(() => {
    if (selectedPipelineId.value && selectedView.value) {
      syncToSelection();
    }
  });

  onUnmounted(() => {
    disposed = true;
    stopPolling();
  });

  return {
    // state
    selectedView,
    selectedPipelineId,
    eventLog,
    pollingActive,
    pollError,
    busy,
    actionError,
    deleteNeedsForce,
    flags,
    // actions
    run,
    pause,
    resume,
    cancel,
    remove,
    create,
    dismissForceDelete,
    startPolling,
    stopPolling,
  };
}
