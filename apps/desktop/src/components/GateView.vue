<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import GateViewContent from './GateViewContent.vue';
import GateViewFilters from './GateViewFilters.vue';
import GateViewStatus from './GateViewStatus.vue';
import GateViewToolbar from './GateViewToolbar.vue';
import GateCreateTaskDialog from './GateCreateTaskDialog.vue';
import ProvenanceBreadcrumbs from './ProvenanceBreadcrumbs.vue';
import { useProvenanceNavigation } from '../composables/useProvenanceNavigation';
import { useGateMcp } from '../composables/useGateMcp';
import { useGateRegistry } from '../composables/useGateRegistry';
import { useProvenanceStore } from '../stores/provenance';
import { useGateTaskFiltering } from '../composables/useGateTaskFiltering';
import { findTaskByLineageId, gateTaskSelectionKey, useGateTaskDetail } from '../composables/useGateTaskDetail';
import { useAppShellStore } from '../stores/app-shell';
import { useGateKeyboard } from '../composables/useGateKeyboard';
import type { GateSnapshot, GateTaskHistoryEntry } from '../lib/gate';
import { createGateSnapshot } from '../lib/gate';

const props = defineProps<{ cwd: string }>();
const registry = useGateRegistry(() => props.cwd);
const gateMcp = useGateMcp();
const provenance = useProvenanceStore();
const { navigateTo } = useProvenanceNavigation();

const snapshot = ref<GateSnapshot | null>(null);
const isLoading = ref(false);
const error = ref('');
const lastRefreshLabel = ref('');
const createTaskOpen = ref(false);
const toolbarRef = ref<{ searchInputEl: HTMLElement | null; openHelp?: () => void } | null>(null);
const contentPanelRef = ref<{ openAppend: () => void; openPass: () => void; openFail: () => void } | null>(null);
const isActive = computed(() => useAppShellStore().activeTab === 'gate');
const lastUpdatedByTaskKey = ref<Record<string, string | null>>({});

const gateRoot = computed(() => registry.selectedGateRoot.value);
const enrichedSnapshot = computed<GateSnapshot | null>(() => {
  const current = snapshot.value;
  if (!current) return null;
  return {
    ...current,
    tasks: current.tasks.map((task) => ({
      ...task,
      lastUpdatedAt: task.lastUpdatedAt
        ?? task.last_updated_at
        ?? lastUpdatedByTaskKey.value[gateTaskSelectionKey(task)]
        ?? null,
    })),
  };
});

const filtering = useGateTaskFiltering(enrichedSnapshot);
const detail = useGateTaskDetail({ snapshot: enrichedSnapshot, gateRoot, gateMcp, provenance });

let snapshotRequestToken = 0;
let stopStatusSubscription: (() => void) | null = null;

const hasGateRoot = computed(() => gateRoot.value.length > 0);
const uiError = computed(() => registry.error.value || error.value || '');
const totalCount = computed(() => (snapshot.value?.tasks ?? []).length);
const todoCount = computed(() => (snapshot.value?.tasks ?? []).filter((task) => task.location === 'todo').length);
const doneCount = computed(() => (snapshot.value?.tasks ?? []).filter((task) => task.location === 'done').length);
const showStatus = computed(() => Boolean(
  uiError.value
  || !hasGateRoot.value
  || (isLoading.value && !snapshot.value)
  || (snapshot.value && !snapshot.value.exists),
));
const shownCount = computed(() => filtering.visibleTasks.value.length);

function formatRefreshTime() {
  return `Last synced: ${new Date().toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })}`;
}

function resetLastUpdatedCache() {
  lastUpdatedByTaskKey.value = {};
}

function latestHistoryTimestamp(history: GateTaskHistoryEntry[]) {
  let latestValue: string | null = null;
  let latestTime = Number.NEGATIVE_INFINITY;
  for (const entry of history) {
    if (!entry.timestamp) continue;
    const time = Date.parse(entry.timestamp);
    if (Number.isNaN(time)) {
      if (latestValue === null) latestValue = entry.timestamp;
      continue;
    }
    if (time >= latestTime) {
      latestTime = time;
      latestValue = entry.timestamp;
    }
  }
  return latestValue;
}

function recordActiveTaskLastUpdated() {
  const task = detail.activeTask.value;
  if (!task) return;
  const key = gateTaskSelectionKey(task);
  const value = latestHistoryTimestamp(detail.selectedTaskHistory.value);
  const cache = lastUpdatedByTaskKey.value;
  if (Object.prototype.hasOwnProperty.call(cache, key) && cache[key] === value) return;
  lastUpdatedByTaskKey.value = { ...cache, [key]: value };
}

function navigateTask(delta: 1 | -1) {
  const tasks = filtering.visibleTasks.value;
  const cur = tasks.findIndex((t) => gateTaskSelectionKey(t) === detail.selectedTaskKey.value);
  if (tasks.length) void detail.selectTask(tasks[Math.max(0, Math.min(tasks.length - 1, cur < 0 ? (delta > 0 ? 0 : tasks.length - 1) : cur + delta))]!);
}
function stopStatusWatch() {
  stopStatusSubscription?.();
  stopStatusSubscription = null;
}

async function refreshSnapshot(background = false) {
  const root = gateRoot.value;
  const requestId = ++snapshotRequestToken;

  if (!root) {
    snapshot.value = null;
    error.value = '';
    lastRefreshLabel.value = '';
    resetLastUpdatedCache();
    detail.clearSelection();
    return;
  }

  if (!background) isLoading.value = true;
  error.value = '';

  try {
    const tasks = await gateMcp.status({ root, includeDone: true });
    const nextSnapshot = createGateSnapshot(root, Array.isArray(tasks) ? tasks : []);
    if (requestId !== snapshotRequestToken || gateRoot.value !== root) return;
    snapshot.value = nextSnapshot;
    resetLastUpdatedCache();
    lastRefreshLabel.value = formatRefreshTime();
    if (!detail.selectedTaskId.value) return;
    const resolved = detail.activeTask.value;
    if (resolved) detail.selectedTaskKey.value = gateTaskSelectionKey(resolved);
    await detail.loadTaskDetail(
      resolved,
      { preserveExisting: background, showLoading: !background },
    );
  } catch (loadError) {
    if (requestId !== snapshotRequestToken || gateRoot.value !== root) return;
    error.value = loadError instanceof Error ? loadError.message : String(loadError);
  } finally {
    if (!background && requestId === snapshotRequestToken && gateRoot.value === root) {
      isLoading.value = false;
    }
  }
}

async function startStatusWatch(root: string) {
  stopStatusWatch();
  if (!root || gateRoot.value !== root) return;
  gateMcp.startPolling(root, () => refreshSnapshot(true));
  stopStatusSubscription = () => gateMcp.stopPolling();
}

async function handleGateRootChange(root: string) {
  await registry.selectRoot(root);
}

async function handleRemoveCurrentGate() {
  const removed = await registry.removeSelectedRoot();
  if (!removed) return;
  detail.clearSelection();
}

async function handleGateActionComplete() {
  await refreshSnapshot(true);
}

async function handleTaskCreated(taskId: string) {
  await refreshSnapshot();
  const task = snapshot.value?.tasks.find((item) => item.taskId === taskId);
  if (task) await detail.selectTask(task);
}

watch(gateRoot, async (root) => {
  detail.selectedTaskId.value = '';
  detail.selectedTaskKey.value = '';
  await refreshSnapshot();
  await startStatusWatch(root);
}, { immediate: true });

watch(detail.selectedTaskHistory, () => recordActiveTaskLastUpdated());

watch([() => provenance.pendingGateTaskId, enrichedSnapshot], async ([pendingTaskId]) => {
  if (!pendingTaskId) return;
  const task = findTaskByLineageId(enrichedSnapshot.value?.tasks ?? [], pendingTaskId);
  if (task) {
    await detail.selectTask(task);
    provenance.clearQueuedGateTaskSelection();
  }
});

onBeforeUnmount(() => { stopStatusWatch(); });
useGateKeyboard({
  isActive,
  selectedTaskKey: detail.selectedTaskKey,
  searchInputEl: computed(() => toolbarRef.value?.searchInputEl ?? null),
  onNext: () => navigateTask(1),
  onPrev: () => navigateTask(-1),
  onOpen: () => { const t = detail.activeTask.value; if (t) void detail.selectTask(t); },
  onClose: () => detail.clearSelection(),
  onNew: () => { createTaskOpen.value = true; },
  onPass: () => contentPanelRef.value?.openPass(),
  onFail: () => contentPanelRef.value?.openFail(),
  onAppend: () => contentPanelRef.value?.openAppend(),
  onRefresh: () => void refreshSnapshot(),
  onHelp: () => toolbarRef.value?.openHelp?.(),
});</script>

<template>
  <div class="flex flex-1 flex-col min-h-0 p-4 gap-3" role="region" aria-label="Gate tasks">
      <ProvenanceBreadcrumbs
        current-kind="gate"
        :current-id="detail.currentGateLineageId.value"
        :current-label="detail.activeTask.value?.taskId ?? undefined"
        :links="detail.provenanceLinks.value"
        @navigate="navigateTo"
      />
      <GateViewToolbar
        ref="toolbarRef"
        :has-gate-root="hasGateRoot"
        :todo-count="todoCount"
        :done-count="doneCount"
        :total-count="totalCount"
        :shown-count="shownCount"
        :last-refresh-label="lastRefreshLabel"
        :selected-gate-root="gateRoot"
        :selected-location="filtering.selectedLocation.value"
        :gate-roots="registry.gateRoots.value"
        :is-ready="registry.isReady.value"
        :is-loading="isLoading"
        :tasks="snapshot?.tasks ?? []"
        :sprint-options="filtering.sprintOptions.value"
        :selected-sprints="filtering.selectedSprints.value"
        :search-text="filtering.searchText.value"
        @update:selected-location="(value) => { filtering.selectedLocation.value = value; }"
        @toggle-sprint="filtering.toggleSprint"
        @update:search-text="(v) => { filtering.searchText.value = v; }"
        @change-root="handleGateRootChange"
        @browse="registry.browseForRoot"
        @remove="handleRemoveCurrentGate"
        @refresh="() => refreshSnapshot()"
        @new-task="createTaskOpen = true"
      />
      <GateCreateTaskDialog
        v-if="hasGateRoot"
        v-model:open="createTaskOpen"
        :root="gateRoot"
        :tasks="snapshot?.tasks ?? []"
        @created="handleTaskCreated"
      />
      <GateViewFilters
        v-model:searchText="filtering.searchText.value"
        v-model:selectedProcess="filtering.selectedProcess.value"
        v-model:selectedState="filtering.selectedState.value"
        v-model:readinessFilter="filtering.readinessFilter.value"
        v-model:issuesOnly="filtering.issuesOnly.value"
        :process-options="filtering.processOptions.value"
        :state-options="filtering.stateOptions.value"
      />
      <GateViewStatus
        v-if="showStatus"
        :ui-error="uiError"
        :has-gate-root="hasGateRoot"
        :is-loading="isLoading"
        :snapshot="snapshot"
        :selected-gate-root="gateRoot"
      />
      <GateViewContent
        ref="contentPanelRef"
        v-else
        :tasks="filtering.visibleTasks.value"
        :selected-task-key="detail.selectedTaskKey.value"
        :sort-column="filtering.sortColumn.value"
        :sort-direction="filtering.sortDirection.value"
        :active-task="detail.activeTask.value"
        :body="detail.selectedTaskBody.value"
        :history="detail.selectedTaskHistory.value"
        :body-loading="detail.detailBodyLoading.value"
        :history-loading="detail.detailHistoryLoading.value"
        :body-error="detail.detailBodyError.value"
        :history-error="detail.detailHistoryError.value"
        :root="gateRoot"
        @sort="filtering.toggleSort"
        @select="detail.selectTask"
        @close="detail.clearSelection"
        @action-complete="handleGateActionComplete"
      />
  </div>
</template>
