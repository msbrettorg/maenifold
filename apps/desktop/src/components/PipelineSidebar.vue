<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { AlertTriangle, Plus, RefreshCw } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import PipelineSidebarItem from './PipelineSidebarItem.vue';
import { usePipelineStore } from '../stores/pipeline';
import { McpCapabilityUnsupportedError, useMcpResources } from '../composables/useMcpResources';
import { useConfirmDialogStore } from '../stores/confirm-dialog';
import type { PipelineListItem } from '../lib/pipeline-types';

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

onBeforeUnmount(() => {
  clearSessionSubscription();
});
</script>

<template>
  <aside
    class="flex h-full w-full min-w-0 flex-col bg-bg-sidebar"
    role="complementary"
    aria-label="Pipeline sidebar"
  >
    <div
      v-if="!hideHeader"
      class="flex items-center justify-between gap-2 border-b border-border-subtle px-3 py-3"
    >
      <h3 class="m-0 text-sm font-semibold text-text-primary">Pipelines</h3>
      <div class="flex items-center gap-1">
        <span
          v-if="resourceError"
          class="inline-flex items-center text-warning"
          :title="resourceError"
          aria-label="Pipeline resource fallback active"
        >
          <AlertTriangle class="size-4" aria-hidden="true" />
        </span>
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          class="text-text-secondary hover:text-text-primary"
          title="Refresh"
          aria-label="Refresh pipelines"
          @click="handleRefresh"
        >
          <RefreshCw class="size-4" aria-hidden="true" />
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          class="gap-1 text-text-secondary hover:text-text-primary"
          title="New pipeline"
          aria-label="New pipeline"
          @click="handleNew"
        >
          <Plus class="size-4" aria-hidden="true" />
          <span>New</span>
        </Button>
      </div>
    </div>

    <div
      v-if="pipelineItems.length === 0"
      class="grid flex-1 place-items-center p-8 text-center text-sm text-text-secondary"
    >
      <p>No pipelines yet</p>
    </div>

    <ScrollArea v-else class="min-h-0 flex-1">
      <ul class="m-0 flex list-none flex-col gap-2 p-3">
        <PipelineSidebarItem
          v-for="item in pipelineItems"
          :key="item.pipelineId"
          :item="item"
          :active="currentPipelineId === item.pipelineId"
          @select="handleSelect"
          @delete="handleDelete"
        />
      </ul>
    </ScrollArea>
  </aside>
</template>
