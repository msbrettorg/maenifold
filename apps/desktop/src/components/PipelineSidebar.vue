<!-- audit-ok: max-loc -->
<script setup lang="ts">
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { usePipelineStore } from '../stores/pipeline';
import { McpCapabilityUnsupportedError, useMcpResources } from '../composables/useMcpResources';
import { useConfirmDialogStore } from '../stores/confirm-dialog';
import type { PipelineListItem, PipelineStatus } from '../lib/pipeline-types';

const props = withDefaults(defineProps<{
  hideHeader?: boolean;
  filterQuery?: string;
}>(), {
  hideHeader: false,
  filterQuery: '',
});

const emit = defineEmits<{
  (e: 'pipeline-selected', pipelineId: string): void;
}>();

const store = usePipelineStore();
const resources = useMcpResources();
const confirmDialogStore = useConfirmDialogStore();
const { connected, pipelines, currentPipelineId, hasDirtyDraft } = storeToRefs(store);
const resourceError = ref<string | null>(null);
let disposeSessionSubscription: (() => void) | null = null;

const pipelineItems = computed(() => {
  const q = props.filterQuery.trim().toLowerCase();
  if (!q) return pipelines.value;
  return pipelines.value.filter((p) => p.pipelineId.toLowerCase().includes(q));
});

function normalizePipelineSessionsPayload(payload: unknown): PipelineListItem[] {
  if (Array.isArray(payload)) {
    return payload as PipelineListItem[];
  }
  if (!payload || typeof payload !== 'object') {
    return [];
  }
  const value = payload as { sessions?: PipelineListItem[]; pipelines?: PipelineListItem[] };
  return Array.isArray(value.sessions)
    ? value.sessions
    : (Array.isArray(value.pipelines) ? value.pipelines : []);
}

async function refreshFromPipelineResource() {
  const content = await resources.readResource('pipeline://sessions');
  if (typeof content.text !== 'string') {
    pipelines.value = [];
    return;
  }
  const parsed = JSON.parse(content.text) as unknown;
  pipelines.value = normalizePipelineSessionsPayload(parsed);
}

function isCapabilityUnsupported(error: unknown): boolean {
  return error instanceof McpCapabilityUnsupportedError;
}

function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function wait(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function handleResourceFailure(context: string, error: unknown) {
  if (!isCapabilityUnsupported(error)) {
    resourceError.value = `${context}: ${getErrorMessage(error)}`;
    console.warn(`[PipelineSidebar] ${context}; falling back to store.refreshList()`, error); // audit-ok: no-console
  }
  await store.refreshList();
}

async function ensureSessionSubscription() {
  if (!connected.value || disposeSessionSubscription) {
    return;
  }
  const onResourceUpdate = () => {
    void refreshFromPipelineResource().catch(async (error: unknown) => {
      await handleResourceFailure('Failed to refresh pipeline resource from MCP update', error);
    });
  };

  try {
    disposeSessionSubscription = await resources.subscribeResource('pipeline://sessions', onResourceUpdate);
  } catch (error) {
    if (isCapabilityUnsupported(error)) {
      throw error;
    }

    await wait(2000);
    disposeSessionSubscription = await resources.subscribeResource('pipeline://sessions', onResourceUpdate);
  }
}

function clearSessionSubscription() {
  if (!disposeSessionSubscription) {
    return;
  }
  disposeSessionSubscription();
  disposeSessionSubscription = null;
}

onMounted(async () => {
  if (!connected.value) {
    return;
  }
  try {
    await refreshFromPipelineResource();
    await ensureSessionSubscription();
    resourceError.value = null;
  } catch (error) {
    await handleResourceFailure('Failed to sync pipeline sessions from MCP resource', error);
  }
});

watch(connected, async (c) => {
  if (!c) {
    clearSessionSubscription();
    return;
  }
  try {
    await refreshFromPipelineResource();
    await ensureSessionSubscription();
    resourceError.value = null;
  } catch (error) {
    await handleResourceFailure('Failed to sync pipeline sessions after MCP reconnect', error);
  }
});

async function handleRefresh() {
  if (!connected.value) {
    return;
  }
  try {
    await refreshFromPipelineResource();
    resourceError.value = null;
  } catch (error) {
    await handleResourceFailure('Failed to refresh pipeline sessions from MCP resource', error);
  }
}

function formatStatus(status: PipelineStatus): string {
  return status.charAt(0).toUpperCase() + status.slice(1);
}

function progressPct(item: PipelineListItem): number {
  return item.nodeCount > 0 ? (item.completedCount / item.nodeCount) * 100 : 0;
}

async function handleSelect(pipelineId: string) {
  if (currentPipelineId.value === pipelineId) return;
  if (hasDirtyDraft.value) {
    const confirmed = await confirmDialogStore.confirm(
      'Discard unsaved pipeline draft?',
      'Your unsaved draft changes will be lost.',
    );
    if (!confirmed) {
      return;
    }
  }
  const loaded = await store.loadPipeline(pipelineId);
  if (loaded) emit('pipeline-selected', pipelineId);
}

async function handleNew() {
  if (hasDirtyDraft.value) {
    const confirmed = await confirmDialogStore.confirm(
      'Discard unsaved pipeline draft?',
      'Your unsaved draft changes will be lost.',
    );
    if (!confirmed) {
      return;
    }
  }
  store.newPipeline();
}

async function handleDelete(item: PipelineListItem, event: Event) {
  event.stopPropagation();
  const confirmed = await confirmDialogStore.confirm(
    `Delete pipeline "${item.pipelineId}"?`,
    'This action cannot be undone.',
  );
  if (!confirmed) return;
  await store.deletePipelineById(item.pipelineId);
}

function beadClass(status: PipelineStatus): string {
  switch (status) {
    case 'active': return 'bg-[var(--amber)]';
    case 'completed':
    case 'paused': return 'bg-[var(--sage)]';
    case 'abandoned':
    case 'stuck': return 'bg-[var(--terracotta)]';
    case 'cancelled':
    default: return 'bg-[var(--text-secondary)]';
  }
}

function statusTextClass(status: PipelineStatus): string {
  switch (status) {
    case 'active': return 'text-[var(--color-status-active)]';
    case 'completed': return 'text-[var(--color-status-completed)]';
    case 'stuck': return 'text-[var(--color-status-stuck)]';
    case 'paused': return 'text-[var(--color-status-ready)]';
    case 'abandoned':
    case 'cancelled': return 'text-[var(--color-status-abandoned)]';
    default: return 'text-[var(--color-status-pending)]';
  }
}

onBeforeUnmount(() => {
  clearSessionSubscription();
});
</script>

<template>
  <aside
    class="flex h-full w-full min-w-0 flex-col bg-[var(--surface)]"
    role="complementary"
    aria-label="Pipeline sidebar"
  >
    <div v-if="!hideHeader" class="flex items-center justify-between p-4">
      <h3 class="m-0 font-display text-[length:var(--text-title)] font-medium text-[var(--color-text-primary)]">Pipelines</h3>
      <div class="flex items-center gap-1">
        <span
          v-if="resourceError"
          class="text-[length:var(--text-xs)] text-[var(--amber)]"
          :title="resourceError"
          aria-label="Pipeline resource fallback active"
        >⚠</span>
        <Button variant="ghost" size="sm" type="button" title="Refresh" @click="handleRefresh">↻</Button>
        <Button variant="ghost" size="sm" type="button" @click="handleNew">+ New</Button>
      </div>
    </div>

    <div
      v-if="pipelineItems.length === 0"
      class="grid flex-1 place-items-center p-8 text-center text-[var(--text-secondary)]"
    >
      <p>No pipelines yet</p>
    </div>

    <ScrollArea v-else class="min-h-0 flex-1">
      <ul class="m-0 flex list-none flex-col gap-2 p-3">
        <li
          v-for="item in pipelineItems"
          :key="item.pipelineId"
          class="group flex items-start justify-between gap-3 rounded-[var(--radius-4)] border-l-4 border-transparent bg-[var(--surface-container)] p-3 transition-[background,border-color] duration-150 hover:bg-[var(--surface-high)]"
          :class="currentPipelineId === item.pipelineId ? '!border-l-[var(--color-brand)] bg-[var(--surface-high)]' : ''"
        >
          <Button
            type="button"
            variant="ghost"
            class="flex min-w-0 flex-1 cursor-pointer items-start gap-2.5 border-0 bg-transparent p-0 text-left text-inherit [font:inherit] focus-visible:rounded-[var(--radius-4)] focus-visible:outline-2 focus-visible:outline-[var(--color-brand)] focus-visible:outline-offset-[3px]"
            :data-pipeline-id="item.pipelineId"
            data-pipeline-item
            :aria-current="currentPipelineId === item.pipelineId ? 'page' : undefined"
            @click="handleSelect(item.pipelineId)"
          >
            <span
              class="mt-1.5 h-1.5 w-1.5 flex-shrink-0 rounded-[var(--radius)]"
              :class="beadClass(item.status)"
            />

            <div class="flex min-w-0 flex-1 flex-col gap-1">
              <span
                class="overflow-hidden text-ellipsis whitespace-nowrap font-[var(--weight-display)] text-[var(--color-text-primary)]"
                :title="item.pipelineId"
              >{{ item.pipelineId }}</span>
              <span class="text-[length:var(--text-xs)] text-[var(--text-secondary)]">{{ item.completedCount }}/{{ item.nodeCount }} nodes</span>
              <div class="h-[3px] overflow-hidden rounded-[var(--radius-4)] bg-[var(--color-bg-app)]">
                <!-- audit-ok: dynamic-binding (progress bar width computed from pipeline state) -->
                <div
                  class="h-full rounded-[var(--radius-4)] bg-[var(--color-brand)] transition-[width] duration-300"
                  :style="{ width: `${progressPct(item)}%` }"
                />
              </div>
            </div>
          </Button>

          <div class="flex min-w-0 flex-col items-end gap-1.5">
            <span
              class="text-[length:var(--text-xs)] font-[var(--weight-display)]"
              :class="statusTextClass(item.status)"
            >
              {{ formatStatus(item.status) }}
            </span>
            <Button
              class="h-6 cursor-pointer rounded-[4px] px-2 text-xs leading-none opacity-0 transition-opacity duration-150 group-hover:opacity-100 group-focus-within:opacity-100"
              variant="destructive"
              size="sm"
              type="button"
              title="Delete pipeline"
              aria-label="Delete pipeline"
              @click="handleDelete(item, $event)"
            >×</Button>
          </div>
        </li>
      </ul>
    </ScrollArea>
  </aside>
</template>
