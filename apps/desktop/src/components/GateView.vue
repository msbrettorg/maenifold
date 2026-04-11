<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { readTextFile } from '@tauri-apps/plugin-fs';
import GateTaskDetail from './GateTaskDetail.vue';
import GateTaskTable from './GateTaskTable.vue';
import GateViewFilters from './GateViewFilters.vue';
import GateViewToolbar from './GateViewToolbar.vue';
import { useGateMcp } from '../composables/useGateMcp';
import { useGateRegistry } from '../composables/useGateRegistry';
import type { GateSnapshot, GateTask, GateTaskHistoryEntry } from '../lib/gate';
import { parseGateTaskBodyContent } from '../lib/gate-task-body';

const props = defineProps<{ cwd: string }>();
const registry = useGateRegistry(() => props.cwd);
const gate = useGateMcp();

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
const detailLoading = ref(false);
const detailError = ref('');
const activeWatcherRoot = ref('');
let detailRequestToken = 0;

type SortColumn = 'id' | 'name' | 'process' | 'progress';
type SortDirection = 'asc' | 'desc';
const sortColumn = ref<SortColumn>('id');
const sortDirection = ref<SortDirection>('asc');

const hasGateRoot = computed(() => registry.selectedGateRoot.value.length > 0);
const uiError = computed(() => registry.error.value || error.value || gate.error.value || '');
const processOptions = computed(() => uniqueValues((snapshot.value?.tasks ?? []).map(processValue)));
const stateOptions = computed(() => uniqueValues((snapshot.value?.tasks ?? []).map((task) => task.state)));
const locationOptions = computed(() => uniqueValues((snapshot.value?.tasks ?? []).map((task) => task.location)));
const activeTask = computed(() => snapshot.value?.tasks.find((task) => task.taskId === selectedTaskId.value) ?? null);
const totalCount = computed(() => (snapshot.value?.tasks ?? []).length);
const todoCount = computed(() => (snapshot.value?.tasks ?? []).filter((task) => task.location === 'todo').length);
const doneCount = computed(() => (snapshot.value?.tasks ?? []).filter((task) => task.location === 'done').length);
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

function uniqueValues(values: string[]) {
  return [...new Set(values.filter(Boolean))].sort((left, right) => left.localeCompare(right));
}

function processValue(task: GateTask) {
  return task.pipeline?.trim() || 'unbound';
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
  return new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
}

function stopWatcher() {
  activeWatcherRoot.value = '';
  gate.stopPolling();
}

async function loadTaskDetail(task: GateTask | null, options: { preserveExisting?: boolean; showLoading?: boolean } = {}) {
  const requestToken = ++detailRequestToken;
  const preserveExisting = options.preserveExisting ?? false;
  const showLoading = options.showLoading ?? false;

  if (!task) {
    selectedTaskBody.value = '';
    selectedTaskHistory.value = [];
    detailError.value = '';
    detailLoading.value = false;
    return;
  }

  if (!preserveExisting) {
    selectedTaskBody.value = '';
    selectedTaskHistory.value = [];
    detailError.value = '';
  }

  if (showLoading) {
    detailLoading.value = true;
  }
  try {
    const [history, rawBody] = await Promise.all([
      gate.readTaskHistory(task.taskId, registry.selectedGateRoot.value),
      readTextFile(task.path),
    ]);

    if (requestToken !== detailRequestToken || selectedTaskId.value !== task.taskId) {
      return;
    }

    selectedTaskHistory.value = history;
    selectedTaskBody.value = parseGateTaskBodyContent(rawBody);
    detailError.value = '';
  } catch (loadError) {
    if (requestToken !== detailRequestToken || selectedTaskId.value !== task.taskId) {
      return;
    }

    detailError.value = loadError instanceof Error ? loadError.message : String(loadError);
  } finally {
    if (requestToken === detailRequestToken) {
      detailLoading.value = false;
    }
  }
}

async function refreshSnapshot(background = false) {
  if (!hasGateRoot.value) {
    snapshot.value = null;
    error.value = '';
    lastRefreshLabel.value = '';
    return;
  }

  if (!background) isLoading.value = true;
  error.value = '';

  try {
    const nextSnapshot = await gate.loadSnapshot(registry.selectedGateRoot.value);
    snapshot.value = nextSnapshot;
    lastRefreshLabel.value = formatRefreshTime();
    if (!selectedTaskId.value) return;
    await loadTaskDetail(nextSnapshot.tasks.find((task) => task.taskId === selectedTaskId.value) ?? null, {
      preserveExisting: background,
      showLoading: !background,
    });
  } catch (loadError) {
    error.value = loadError instanceof Error ? loadError.message : String(loadError);
  } finally {
    if (!background) isLoading.value = false;
  }
}

async function startWatcher(root: string) {
  if (!root) {
    stopWatcher();
    return;
  }
  if (activeWatcherRoot.value === root) return;

  stopWatcher();
  activeWatcherRoot.value = root;
  gate.startPolling(root, () => refreshSnapshot(true));
}

async function handleTaskSelect(task: GateTask) {
  selectedTaskId.value = task.taskId;
  await loadTaskDetail(task, { showLoading: true });
}

async function handleGateRootChange(root: string) {
  await registry.selectRoot(root);
}

async function handleRemoveCurrentGate() {
  const removed = await registry.removeSelectedRoot();
  if (!removed) return;
  clearSelection();
}

function clearSelection() {
  detailRequestToken += 1;
  selectedTaskId.value = '';
  selectedTaskBody.value = '';
  selectedTaskHistory.value = [];
  detailLoading.value = false;
  detailError.value = '';
}

watch(() => registry.selectedGateRoot.value, async (root) => {
  selectedTaskId.value = '';
  await refreshSnapshot();
  await startWatcher(root);
}, { immediate: true });

watch(processOptions, (options) => { if (selectedProcess.value !== 'all' && !options.includes(selectedProcess.value)) selectedProcess.value = 'all'; });
watch(stateOptions, (options) => { if (selectedState.value !== 'all' && !options.includes(selectedState.value)) selectedState.value = 'all'; });
watch(locationOptions, (options) => { if (selectedLocation.value !== 'all' && !options.includes(selectedLocation.value)) selectedLocation.value = 'all'; });

onBeforeUnmount(() => {
  stopWatcher();
  void gate.disconnect();
});
</script>

<template>
  <section class="gate-view">
    <GateViewToolbar :has-gate-root="hasGateRoot" :todo-count="todoCount" :done-count="doneCount" :total-count="totalCount" :shown-count="shownCount" :last-refresh-label="lastRefreshLabel" :selected-gate-root="registry.selectedGateRoot.value" :gate-roots="registry.gateRoots.value" :is-ready="registry.isReady.value" :is-loading="isLoading" @change-root="handleGateRootChange" @browse="registry.browseForRoot" @remove="handleRemoveCurrentGate" @refresh="() => refreshSnapshot()" />

    <GateViewFilters v-model:searchText="searchText" v-model:selectedProcess="selectedProcess" v-model:selectedState="selectedState" v-model:selectedLocation="selectedLocation" v-model:issuesOnly="issuesOnly" :process-options="processOptions" :state-options="stateOptions" :location-options="locationOptions" />

    <p v-if="uiError" class="gate-state gate-state--error">{{ uiError }}</p>
    <p v-else-if="!hasGateRoot" class="gate-state">No Gate root selected.</p>
    <p v-else-if="isLoading && !snapshot" class="gate-state">Loading gate snapshot…</p>
    <p v-else-if="snapshot && !snapshot.exists" class="gate-state">No .gate/gate.db found in {{ registry.selectedGateRoot.value }}.</p>

    <div v-else class="gate-content">
      <GateTaskTable :tasks="visibleTasks" :selected-task-id="selectedTaskId" :sort-column="sortColumn" :sort-direction="sortDirection" @sort="toggleSort" @select="handleTaskSelect" />
      <GateTaskDetail :task="activeTask" :body="selectedTaskBody" :history="selectedTaskHistory" :loading="detailLoading" :error="detailError" @close="clearSelection" />
    </div>
  </section>
</template>

<style scoped>
.gate-view { display: flex; flex: 1; flex-direction: column; gap: 0.75rem; padding: 1rem; overflow: auto; }
.gate-state { padding: 0.9rem 1rem; color: var(--text-muted); border: 1px solid var(--border-default); border-radius: var(--radius-lg); background: var(--bg-panel); }
.gate-state--error { color: var(--color-status-abandoned); }
.gate-content { display: flex; gap: 1rem; min-height: 0; }
@media (max-width: 1100px) { .gate-content { flex-direction: column; } }
</style>
