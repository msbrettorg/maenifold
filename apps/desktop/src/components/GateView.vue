<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import GateViewContent from './GateViewContent.vue';
import GateViewFilters from './GateViewFilters.vue';
import GateViewStatus from './GateViewStatus.vue';
import GateViewToolbar from './GateViewToolbar.vue';
import ProvenanceBreadcrumbs from './ProvenanceBreadcrumbs.vue';
import { Tabs, TabsList, TabsTrigger } from './ui/tabs';
import { useProvenanceNavigation } from '../composables/useProvenanceNavigation';
import { useMcpResources } from '../composables/useMcpResources';
import { useGateRegistry } from '../composables/useGateRegistry';
import { useProvenanceStore } from '../stores/provenance';
import { useGateTaskFiltering } from '../composables/useGateTaskFiltering';
import { findTaskByLineageId, useGateTaskDetail } from '../composables/useGateTaskDetail';
import { gateResourceUri, parseResourcePayload } from '../lib/gate-resources';
import type { GateSnapshot, GateTask } from '../lib/gate';
import { createGateSnapshot } from '../lib/gate';

const props = defineProps<{ cwd: string }>();
const registry = useGateRegistry(() => props.cwd);
const resources = useMcpResources();
const provenance = useProvenanceStore();
const { navigateTo } = useProvenanceNavigation();

const snapshot = ref<GateSnapshot | null>(null);
const isLoading = ref(false);
const error = ref('');
const lastRefreshLabel = ref('');

const gateRoot = computed(() => registry.selectedGateRoot.value);

const filtering = useGateTaskFiltering(snapshot);
const detail = useGateTaskDetail({ snapshot, gateRoot, resources, provenance });

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
    detail.clearSelection();
    return;
  }

  if (!background) isLoading.value = true;
  error.value = '';

  try {
    const resource = await resources.readResource(gateResourceUri('status', root));
    const statusPayload = parseResourcePayload<GateTask[] | { tasks?: GateTask[] }>(resource);
    const tasks = Array.isArray(statusPayload) ? statusPayload : statusPayload.tasks ?? [];
    const nextSnapshot = createGateSnapshot(root, tasks);
    if (requestId !== snapshotRequestToken || gateRoot.value !== root) return;
    snapshot.value = nextSnapshot;
    lastRefreshLabel.value = formatRefreshTime();
    if (!detail.selectedTaskId.value) return;
    await detail.loadTaskDetail(
      detail.activeTask.value,
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
  const statusUri = gateResourceUri('status', root);
  try {
    const subscription = await resources.subscribeResource(statusUri, () => {
      void refreshSnapshot(true);
    });
    if (gateRoot.value !== root) {
      subscription();
      return;
    }
    stopStatusSubscription = subscription;
  } catch {
    // Resource subscriptions are best-effort; manual refresh still works.
  }
}

async function handleGateRootChange(root: string) {
  await registry.selectRoot(root);
}

async function handleRemoveCurrentGate() {
  const removed = await registry.removeSelectedRoot();
  if (!removed) return;
  detail.clearSelection();
}

watch(gateRoot, async (root) => {
  detail.selectedTaskId.value = '';
  detail.selectedTaskKey.value = '';
  await refreshSnapshot();
  await startStatusWatch(root);
}, { immediate: true });

watch([() => provenance.pendingGateTaskId, snapshot], async ([pendingTaskId]) => {
  if (!pendingTaskId) return;
  const task = findTaskByLineageId(snapshot.value?.tasks ?? [], pendingTaskId);
  if (task) {
    await detail.selectTask(task);
    provenance.clearQueuedGateTaskSelection();
  }
});

onBeforeUnmount(() => {
  stopStatusWatch();
  detail.stopTaskDetailWatchers();
});
</script>

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
        :has-gate-root="hasGateRoot"
        :todo-count="todoCount"
        :done-count="doneCount"
        :total-count="totalCount"
        :shown-count="shownCount"
        :last-refresh-label="lastRefreshLabel"
        :selected-gate-root="gateRoot"
        :gate-roots="registry.gateRoots.value"
        :is-ready="registry.isReady.value"
        :is-loading="isLoading"
        @change-root="handleGateRootChange"
        @browse="registry.browseForRoot"
        @remove="handleRemoveCurrentGate"
        @refresh="() => refreshSnapshot()"
      />
      <Tabs v-model="filtering.selectedLocation.value" class="w-full">
        <TabsList>
          <TabsTrigger value="todo">
            Todo
            <span class="ml-2 text-xs opacity-70">{{ todoCount }}</span>
          </TabsTrigger>
          <TabsTrigger value="done">
            Done
            <span class="ml-2 text-xs opacity-70">{{ doneCount }}</span>
          </TabsTrigger>
          <TabsTrigger value="all">
            All
            <span class="ml-2 text-xs opacity-70">{{ totalCount }}</span>
          </TabsTrigger>
        </TabsList>
      </Tabs>
      <GateViewFilters
        v-model:searchText="filtering.searchText.value"
        v-model:selectedProcess="filtering.selectedProcess.value"
        v-model:selectedState="filtering.selectedState.value"
        v-model:selectedLocation="filtering.selectedLocation.value"
        v-model:issuesOnly="filtering.issuesOnly.value"
        :process-options="filtering.processOptions.value"
        :state-options="filtering.stateOptions.value"
        :location-options="filtering.locationOptions.value"
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
        @sort="filtering.toggleSort"
        @select="detail.selectTask"
        @close="detail.clearSelection"
      />
  </div>
</template>
