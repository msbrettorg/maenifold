<!-- audit-ok: max-loc -->
<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import GateViewContent from './GateViewContent.vue';
import GateViewFilters from './GateViewFilters.vue';
import GateViewStatus from './GateViewStatus.vue';
import GateViewToolbar from './GateViewToolbar.vue';
import ProvenanceBreadcrumbs from './ProvenanceBreadcrumbs.vue';
import { useProvenanceNavigation } from '../composables/useProvenanceNavigation';
import { useMcpResources } from '../composables/useMcpResources';
import { useGateRegistry } from '../composables/useGateRegistry';
import { useProvenanceStore } from '../stores/provenance';
import type { GateSnapshot, GateTask, GateTaskBody, GateTaskHistoryEntry } from '../lib/gate';
import { parseGateTaskBodyContent } from '../lib/gate-task-body';
import { createGateSnapshot } from '../lib/gate';
import type { ResourceContent } from '../composables/mcpTransport';
import { getTaskPipelineLabel } from '../lib/gate-display';

const props = defineProps<{ cwd: string }>();
const registry = useGateRegistry(() => props.cwd);
const resources = useMcpResources();
const provenance = useProvenanceStore();
const { navigateTo } = useProvenanceNavigation();

const snapshot = ref<GateSnapshot | null>(null);
const isLoading = ref(false);
const error = ref('');
const lastRefreshLabel = ref('');
const searchText = ref('');
const selectedProcess = ref('all');
const selectedState = ref('all');
const selectedLocation = ref('todo');
const issuesOnly = ref(false);
const selectedTaskId = ref('');
const selectedTaskBody = ref('');
const selectedTaskHistory = ref<GateTaskHistoryEntry[]>([]);
const detailBodyLoading = ref(false);
const detailHistoryLoading = ref(false);
const detailBodyError = ref('');
const detailHistoryError = ref('');
let detailRequestToken = 0;
let snapshotRequestToken = 0;
let stopStatusSubscription: (() => void) | null = null;
let stopTaskBodySubscription: (() => void) | null = null;
let stopTaskHistorySubscription: (() => void) | null = null;

const resolveTaskBodyContent = (taskBody: GateTaskBody | string) =>
  parseGateTaskBodyContent(typeof taskBody === 'string' ? taskBody : taskBody.markdown);

type SortColumn = 'id' | 'name' | 'process' | 'progress';
type SortDirection = 'asc' | 'desc';
const sortColumn = ref<SortColumn>('id');
const sortDirection = ref<SortDirection>('asc');

const hasGateRoot = computed(() => registry.selectedGateRoot.value.length > 0);
const uiError = computed(() => registry.error.value || error.value || '');
const processOptions = computed(() => uniqueValues((snapshot.value?.tasks ?? []).map(processValue)));
const stateOptions = computed(() => uniqueValues((snapshot.value?.tasks ?? []).map((task) => task.state)));
const locationOptions = computed(() => uniqueValues((snapshot.value?.tasks ?? []).map((task) => task.location)));
const activeTask = computed(() => snapshot.value?.tasks.find((task) => task.taskId === selectedTaskId.value) ?? null);
const totalCount = computed(() => (snapshot.value?.tasks ?? []).length);
const todoCount = computed(() => (snapshot.value?.tasks ?? []).filter((task) => task.location === 'todo').length);
const doneCount = computed(() => (snapshot.value?.tasks ?? []).filter((task) => task.location === 'done').length);
const showStatus = computed(() => Boolean(uiError.value || !hasGateRoot.value || (isLoading.value && !snapshot.value) || (snapshot.value && !snapshot.value.exists)));
const visibleTasks = computed(() => {
  const search = searchText.value.trim().toLowerCase();

  return [...(snapshot.value?.tasks ?? [])]
    .filter((task) => {
      const haystack = [
        task.taskId,
        task.title,
        task.path,
        task.issue,
        processValue(task),
        task.state,
        task.location,
      ].join('\n').toLowerCase();

      return (!search || haystack.includes(search))
        && (selectedProcess.value === 'all' || processValue(task) === selectedProcess.value)
        && (selectedState.value === 'all' || task.state === selectedState.value)
        && (selectedLocation.value === 'all' || task.location === selectedLocation.value)
        && (!issuesOnly.value || Boolean(task.issue));
    })
    .sort((left, right) => {
      const direction = sortDirection.value === 'asc' ? 1 : -1;
      const compare = {
        id: left.taskId.localeCompare(right.taskId, undefined, { numeric: true }),
        name: left.title.localeCompare(right.title),
        process: processValue(left).localeCompare(processValue(right)),
        progress: progressRank(left) - progressRank(right),
      }[sortColumn.value];

      return compare * direction || left.taskId.localeCompare(right.taskId, undefined, { numeric: true });
    });
});
const shownCount = computed(() => visibleTasks.value.length);
function firstNonEmptyId(...values: Array<string | null | undefined>): string | null {
  for (const value of values) {
    if (typeof value !== 'string') {
      continue;
    }
    const trimmed = value.trim();
    if (trimmed.length > 0) {
      return trimmed;
    }
  }
  return null;
}

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

function uniqueValues(values: string[]) {
  return [...new Set(values.filter(Boolean))].sort((left, right) => left.localeCompare(right));
}

function processValue(task: GateTask) {
  return getTaskPipelineLabel(task);
}

function progressRank(task: GateTask) {
  if (task.location === 'done') return task.stages.length || 1;
  if (!task.stages.length) return task.state === 'pending' ? 0 : 1;
  const index = task.stages.indexOf(task.state);
  return index < 0 ? 0 : index;
}

function toggleSort(column: SortColumn) {
  if (sortColumn.value === column) {
    sortDirection.value = sortDirection.value === 'asc' ? 'desc' : 'asc';
    return;
  }
  sortColumn.value = column;
  sortDirection.value = 'asc';
}

function formatRefreshTime() {
  return `Last synced: ${new Date().toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })}`;
}

function gateResourceUri(path: string, root: string) {
  if (!root) {
    return `gate://${path}`;
  }
  return `gate://${path}?root=${encodeURIComponent(root)}`;
}

function parseResourcePayload<T>(content: ResourceContent): T {
  const text = content.text;
  if (!text) {
    throw new Error(`Resource ${content.uri} returned no text payload.`);
  }
  return JSON.parse(text) as T;
}

function stopStatusWatch() {
  stopStatusSubscription?.();
  stopStatusSubscription = null;
}

function stopTaskDetailWatchers() {
  stopTaskBodySubscription?.();
  stopTaskBodySubscription = null;
  stopTaskHistorySubscription?.();
  stopTaskHistorySubscription = null;
}

async function loadTaskDetail(task: GateTask | null, options: { preserveExisting?: boolean; showLoading?: boolean } = {}) {
  const requestId = ++detailRequestToken;
  const preserveExisting = options.preserveExisting ?? false;
  const showLoading = options.showLoading ?? false;

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

  const isCurrentRequest = () => requestId === detailRequestToken && selectedTaskId.value === task.taskId;
  const root = registry.selectedGateRoot.value;

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
      if (isCurrentRequest()) {
        detailBodyLoading.value = false;
      }
    });

  const historyRequest = resources.readResource(historyUri)
    .then((resource) => {
      if (!isCurrentRequest()) return;
      const history = parseResourcePayload<GateTaskHistoryEntry[]>(resource);
      selectedTaskHistory.value = history;
      detailHistoryError.value = '';
    })
    .catch((loadError) => {
      if (!isCurrentRequest()) return;
      selectedTaskHistory.value = [];
      detailHistoryError.value = loadError instanceof Error ? loadError.message : String(loadError);
    })
    .finally(() => {
      if (isCurrentRequest()) {
        detailHistoryLoading.value = false;
      }
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

function clearSelection() {
  detailRequestToken += 1;
  selectedTaskId.value = '';
  selectedTaskBody.value = '';
  selectedTaskHistory.value = [];
  detailBodyLoading.value = false;
  detailHistoryLoading.value = false;
  detailBodyError.value = '';
  detailHistoryError.value = '';
  stopTaskDetailWatchers();
}

async function refreshSnapshot(background = false) {
  const root = registry.selectedGateRoot.value;
  const requestId = ++snapshotRequestToken;

  if (!root) {
    snapshot.value = null;
    error.value = '';
    lastRefreshLabel.value = '';
    clearSelection();
    return;
  }

  if (!background) isLoading.value = true;
  error.value = '';

  try {
    const statusUri = gateResourceUri('status', root);
    const resource = await resources.readResource(statusUri);
    const statusPayload = parseResourcePayload<GateTask[] | { tasks?: GateTask[] }>(resource);
    const tasks = Array.isArray(statusPayload) ? statusPayload : statusPayload.tasks ?? [];
    const nextSnapshot = createGateSnapshot(root, tasks);
    if (requestId !== snapshotRequestToken || registry.selectedGateRoot.value !== root) {
      return;
    }
    snapshot.value = nextSnapshot;
    lastRefreshLabel.value = formatRefreshTime();
    if (!selectedTaskId.value) return;
    await loadTaskDetail(nextSnapshot.tasks.find((task) => task.taskId === selectedTaskId.value) ?? null, {
      preserveExisting: background,
      showLoading: !background,
    });
  } catch (loadError) {
    if (requestId !== snapshotRequestToken || registry.selectedGateRoot.value !== root) {
      return;
    }
    error.value = loadError instanceof Error ? loadError.message : String(loadError);
  } finally {
    if (!background && requestId === snapshotRequestToken && registry.selectedGateRoot.value === root) {
      isLoading.value = false;
    }
  }
}

async function startStatusWatch(root: string) {
  if (!root) {
    stopStatusWatch();
    return;
  }
  stopStatusWatch();
  if (registry.selectedGateRoot.value !== root) {
    return;
  }
  const statusUri = gateResourceUri('status', root);
  try {
    const subscription = await resources.subscribeResource(statusUri, () => {
      void refreshSnapshot(true);
    });
    if (registry.selectedGateRoot.value !== root) {
      subscription();
      return;
    }
    stopStatusSubscription = subscription;
  } catch {
    // Resource subscriptions are best-effort; manual refresh still works.
  }
}

async function handleTaskSelect(task: GateTask) {
  selectedTaskId.value = task.taskId;
  await loadTaskDetail(task, { showLoading: true });
}

async function handleGateRootChange(root: string) {
  await registry.selectRoot(root);
}

function findTaskByLineageId(tasks: GateTask[], lineageId: string): GateTask | null {
  return tasks.find((task) =>
    task.lineageGateTaskId === lineageId
    || task.lineage_gate_task_id === lineageId
    || task.canonicalTaskId === lineageId
    || task.taskId === lineageId) ?? null;
}

async function handleRemoveCurrentGate() {
  const removed = await registry.removeSelectedRoot();
  if (!removed) return;
  clearSelection();
}

watch(() => registry.selectedGateRoot.value, async (root) => {
  selectedTaskId.value = '';
  await refreshSnapshot();
  await startStatusWatch(root);
}, { immediate: true });

watch(processOptions, (options) => { if (selectedProcess.value !== 'all' && !options.includes(selectedProcess.value)) selectedProcess.value = 'all'; });
watch(stateOptions, (options) => { if (selectedState.value !== 'all' && !options.includes(selectedState.value)) selectedState.value = 'all'; });
watch(locationOptions, (options) => { if (selectedLocation.value !== 'all' && !options.includes(selectedLocation.value)) selectedLocation.value = 'all'; });
watch([() => provenance.pendingGateTaskId, snapshot], async ([pendingTaskId]) => {
  if (!pendingTaskId) {
    return;
  }

  const task = findTaskByLineageId(snapshot.value?.tasks ?? [], pendingTaskId);
  if (task) {
    await handleTaskSelect(task);
    provenance.clearQueuedGateTaskSelection();
  }
});

onBeforeUnmount(() => {
  stopStatusWatch();
  stopTaskDetailWatchers();
});
</script>

<template>
  <section class="flex flex-1 flex-col gap-3 overflow-auto p-4" role="region" aria-label="Gate tasks">
    <ProvenanceBreadcrumbs
      current-kind="gate"
      :current-id="currentGateLineageId"
      :current-label="activeTask?.taskId ?? undefined"
      :links="provenanceLinks"
      @navigate="navigateTo"
    />
    <GateViewToolbar :has-gate-root="hasGateRoot" :todo-count="todoCount" :done-count="doneCount" :total-count="totalCount" :shown-count="shownCount" :last-refresh-label="lastRefreshLabel" :selected-gate-root="registry.selectedGateRoot.value" :gate-roots="registry.gateRoots.value" :is-ready="registry.isReady.value" :is-loading="isLoading" @change-root="handleGateRootChange" @browse="registry.browseForRoot" @remove="handleRemoveCurrentGate" @refresh="() => refreshSnapshot()" />
    <GateViewFilters v-model:searchText="searchText" v-model:selectedProcess="selectedProcess" v-model:selectedState="selectedState" v-model:selectedLocation="selectedLocation" v-model:issuesOnly="issuesOnly" :process-options="processOptions" :state-options="stateOptions" :location-options="locationOptions" />
    <GateViewStatus v-if="showStatus" :ui-error="uiError" :has-gate-root="hasGateRoot" :is-loading="isLoading" :snapshot="snapshot" :selected-gate-root="registry.selectedGateRoot.value" />
    <GateViewContent v-else :tasks="visibleTasks" :selected-task-id="selectedTaskId" :sort-column="sortColumn" :sort-direction="sortDirection" :active-task="activeTask" :body="selectedTaskBody" :history="selectedTaskHistory" :body-loading="detailBodyLoading" :history-loading="detailHistoryLoading" :body-error="detailBodyError" :history-error="detailHistoryError" @sort="toggleSort" @select="handleTaskSelect" @close="clearSelection" />
  </section>
</template>
