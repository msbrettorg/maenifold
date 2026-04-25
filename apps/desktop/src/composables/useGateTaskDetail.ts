import { computed, ref, type Ref } from 'vue';
import type { GateSnapshot, GateTask, GateTaskBody, GateTaskHistoryEntry } from '../lib/gate';
import { parseGateTaskBodyContent } from '../lib/gate-task-body';
import { gateResourceUri, parseResourcePayload } from '../lib/gate-resources';
import type { useMcpResources } from './useMcpResources';
import type { useProvenanceStore } from '../stores/provenance';

type McpResources = ReturnType<typeof useMcpResources>;
type ProvenanceStore = ReturnType<typeof useProvenanceStore>;

interface UseGateTaskDetailOptions {
  snapshot: Ref<GateSnapshot | null>;
  gateRoot: Ref<string>;
  resources: McpResources;
  provenance: ProvenanceStore;
}

interface LoadOptions {
  preserveExisting?: boolean;
  showLoading?: boolean;
}

function firstNonEmptyId(...values: Array<string | null | undefined>): string | null {
  for (const value of values) {
    if (typeof value !== 'string') continue;
    const trimmed = value.trim();
    if (trimmed.length > 0) return trimmed;
  }
  return null;
}

function resolveTaskBodyContent(taskBody: GateTaskBody | string) {
  return parseGateTaskBodyContent(typeof taskBody === 'string' ? taskBody : taskBody.markdown);
}

export function findTaskByLineageId(tasks: GateTask[], lineageId: string): GateTask | null {
  return tasks.find((task) =>
    task.lineageGateTaskId === lineageId
    || task.lineage_gate_task_id === lineageId
    || task.canonicalTaskId === lineageId
    || task.taskId === lineageId) ?? null;
}

export function gateTaskSelectionKey(task: GateTask): string {
  return task.path || `${task.location}:${task.taskId}`;
}

export function useGateTaskDetail(options: UseGateTaskDetailOptions) {
  const { snapshot, gateRoot, resources, provenance } = options;

  const selectedTaskId = ref('');
  const selectedTaskKey = ref('');
  const selectedTaskBody = ref('');
  const selectedTaskHistory = ref<GateTaskHistoryEntry[]>([]);
  const detailBodyLoading = ref(false);
  const detailHistoryLoading = ref(false);
  const detailBodyError = ref('');
  const detailHistoryError = ref('');

  let detailRequestToken = 0;
  let stopTaskBodySubscription: (() => void) | null = null;
  let stopTaskHistorySubscription: (() => void) | null = null;

  const activeTask = computed(() => {
    const tasks = snapshot.value?.tasks ?? [];
    return tasks.find((task) => gateTaskSelectionKey(task) === selectedTaskKey.value)
      ?? tasks.find((task) => task.taskId === selectedTaskId.value)
      ?? null;
  });

  const currentGateLineageId = computed(() =>
    firstNonEmptyId(
      activeTask.value?.lineageGateTaskId,
      activeTask.value?.lineage_gate_task_id,
      activeTask.value?.canonicalTaskId,
      activeTask.value?.taskId,
    ));

  const activeTaskLineageLinks = computed(() =>
    provenance.buildLineageLinks({
      gate: currentGateLineageId.value,
      pipeline: firstNonEmptyId(
        activeTask.value?.lineagePipelineRunId,
        activeTask.value?.lineage_pipeline_run_id,
      ),
      thinking: firstNonEmptyId(
        activeTask.value?.lineageThinkingSessionId,
        activeTask.value?.lineage_thinking_session_id,
      ),
      knowledge: firstNonEmptyId(
        activeTask.value?.lineageMemoryNoteId,
        activeTask.value?.lineage_memory_note_id,
      ),
    }, {
      gate: activeTask.value?.taskId,
    }));

  const provenanceLinks = computed(() =>
    activeTaskLineageLinks.value
      .filter((link) => link.kind !== 'gate' && link.id !== currentGateLineageId.value));

  function stopTaskDetailWatchers() {
    stopTaskBodySubscription?.();
    stopTaskBodySubscription = null;
    stopTaskHistorySubscription?.();
    stopTaskHistorySubscription = null;
  }

  function clearSelection() {
    detailRequestToken += 1;
    selectedTaskId.value = '';
    selectedTaskKey.value = '';
    selectedTaskBody.value = '';
    selectedTaskHistory.value = [];
    detailBodyLoading.value = false;
    detailHistoryLoading.value = false;
    detailBodyError.value = '';
    detailHistoryError.value = '';
    stopTaskDetailWatchers();
  }

  async function loadTaskDetail(task: GateTask | null, loadOptions: LoadOptions = {}) {
    const requestId = ++detailRequestToken;
    const preserveExisting = loadOptions.preserveExisting ?? false;
    const showLoading = loadOptions.showLoading ?? false;

    if (!task) {
      selectedTaskBody.value = '';
      selectedTaskHistory.value = [];
      detailBodyError.value = '';
      detailHistoryError.value = '';
      detailBodyLoading.value = false;
      detailHistoryLoading.value = false;
      return;
    }

    if (!preserveExisting) {
      selectedTaskBody.value = '';
      selectedTaskHistory.value = [];
      detailBodyError.value = '';
      detailHistoryError.value = '';
    }

    detailBodyError.value = '';
    detailHistoryError.value = '';

    if (showLoading || !preserveExisting) {
      detailBodyLoading.value = true;
      detailHistoryLoading.value = true;
    }

    const taskSelectionKey = gateTaskSelectionKey(task);
    const isCurrentRequest = () =>
      requestId === detailRequestToken
      && selectedTaskKey.value === taskSelectionKey;
    const root = gateRoot.value;
    stopTaskDetailWatchers();

    const bodyUri = gateResourceUri(`task/${encodeURIComponent(task.taskId)}`, root);
    const historyUri = gateResourceUri(`history/${encodeURIComponent(task.taskId)}`, root);

    const bodyRequest = resources.readResource(bodyUri)
      .then((resource) => {
        if (!isCurrentRequest()) return;
        const taskBody = parseResourcePayload<GateTaskBody>(resource);
        selectedTaskBody.value = resolveTaskBodyContent(taskBody);
        detailBodyError.value = '';
      })
      .catch((loadError) => {
        if (!isCurrentRequest()) return;
        selectedTaskBody.value = '';
        detailBodyError.value = loadError instanceof Error ? loadError.message : String(loadError);
      })
      .finally(() => {
        if (isCurrentRequest()) detailBodyLoading.value = false;
      });

    const historyRequest = resources.readResource(historyUri)
      .then((resource) => {
        if (!isCurrentRequest()) return;
        selectedTaskHistory.value = parseResourcePayload<GateTaskHistoryEntry[]>(resource);
        detailHistoryError.value = '';
      })
      .catch((loadError) => {
        if (!isCurrentRequest()) return;
        selectedTaskHistory.value = [];
        detailHistoryError.value = loadError instanceof Error ? loadError.message : String(loadError);
      })
      .finally(() => {
        if (isCurrentRequest()) detailHistoryLoading.value = false;
      });

    await Promise.allSettled([bodyRequest, historyRequest]);

    if (isCurrentRequest()) {
      try {
        stopTaskBodySubscription = await resources.subscribeResource(bodyUri, () => {
          void loadTaskDetail(task, { preserveExisting: true, showLoading: false });
        });
        stopTaskHistorySubscription = await resources.subscribeResource(historyUri, () => {
          void loadTaskDetail(task, { preserveExisting: true, showLoading: false });
        });
      } catch {
        // Resource subscriptions are best-effort; manual refresh still works.
      }
    }
  }

  async function selectTask(task: GateTask) {
    selectedTaskId.value = task.taskId;
    selectedTaskKey.value = gateTaskSelectionKey(task);
    await loadTaskDetail(task, { showLoading: true });
  }

  return {
    selectedTaskId,
    selectedTaskKey,
    selectedTaskBody,
    selectedTaskHistory,
    detailBodyLoading,
    detailHistoryLoading,
    detailBodyError,
    detailHistoryError,
    activeTask,
    currentGateLineageId,
    provenanceLinks,
    selectTask,
    loadTaskDetail,
    clearSelection,
    stopTaskDetailWatchers,
  };
}
