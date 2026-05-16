<script setup lang="ts">
import { Button } from '@/components/ui/button';
import { computed, ref, type Component } from 'vue';
import { storeToRefs } from 'pinia';
import { Pause, Play, RotateCcw, Trash2, XCircle } from 'lucide-vue-next';
import type { PipelineStatus } from '../lib/pipeline-types-edge';
import { usePipelineStore } from '../stores/pipeline';
import { useConfirmDialogStore } from '../stores/confirm-dialog';
import PipelineCreateForm from './PipelineCreateForm.vue';

type ActionKey = 'create' | 'run' | 'pause' | 'resume' | 'cancel' | 'rewind' | 'delete';

interface PipelineCreateFormSubmit {
  name?: string;
  description?: string;
}

interface ActionConfig {
  key: ActionKey;
  label: string;
  icon: Component;
  variant: 'default' | 'secondary' | 'destructive';
  class?: string;
  run: () => Promise<unknown>;
}

type PipelineActionStatus = PipelineStatus | 'abandoned';

const store = usePipelineStore();
const confirmDialogStore = useConfirmDialogStore();
const { currentView, designMode, isRunning, localDefinition } = storeToRefs(store);
const busyAction = ref<ActionKey | null>(null);

const hasDraftNodes = computed(() => (localDefinition.value?.nodes?.length ?? 0) > 0);
const showCreateForm = computed(() => designMode.value && hasDraftNodes.value);
const currentStatus = computed<PipelineActionStatus | null>(
  () => (currentView.value?.status as PipelineActionStatus | undefined) ?? null,
);
const rewindableNodeIds = computed(() =>
  currentView.value?.nodes
    .filter((node) => node.status === 'completed' || node.status === 'abandoned' || node.status === 'stuck')
    .map((node) => node.nodeId) ?? [],
);

const actions = computed((): ActionConfig[] => {
  if (designMode.value) {
    return [];
  }

  switch (currentStatus.value) {
    case 'paused':
      return [
        {
          key: 'resume',
          label: 'Resume',
          icon: Play,
          variant: 'default',
          run: () => store.resumePipeline(),
        },
        {
          key: 'cancel',
          label: 'Cancel',
          icon: XCircle,
          variant: 'destructive',
          run: () => store.cancelPipeline(),
        },
      ];
    case 'completed':
      return [
        {
          key: 'rewind',
          label: 'Rewind',
          icon: RotateCcw,
          variant: 'default',
          run: handleRewind,
        },
        {
          key: 'delete',
          label: 'Delete',
          icon: Trash2,
          variant: 'destructive',
          run: handleDelete,
        },
      ];
    case 'abandoned':
      return [
        {
          key: 'rewind',
          label: 'Rewind',
          icon: RotateCcw,
          variant: 'default',
          run: handleRewind,
        },
        {
          key: 'delete',
          label: 'Delete',
          icon: Trash2,
          variant: 'destructive',
          run: handleDelete,
        },
      ];
    case 'stuck':
      return [
        {
          key: 'rewind',
          label: 'Rewind',
          icon: RotateCcw,
          variant: 'default',
          run: handleRewind,
        },
        {
          key: 'cancel',
          label: 'Cancel',
          icon: XCircle,
          variant: 'destructive',
          run: () => store.cancelPipeline(),
        },
        {
          key: 'delete',
          label: 'Delete',
          icon: Trash2,
          variant: 'destructive',
          run: handleDelete,
        },
      ];
    case 'active':
      return isRunning.value
        ? [
            {
              key: 'pause',
              label: 'Pause',
              icon: Pause,
              variant: 'secondary',
              run: () => store.pausePipeline(),
            },
            {
              key: 'cancel',
              label: 'Cancel',
              icon: XCircle,
              variant: 'destructive',
              run: () => store.cancelPipeline(),
            },
          ]
        : [
            ...(rewindableNodeIds.value.length > 0
              ? ([{
                   key: 'rewind' as ActionKey,
                   label: 'Rewind',
                   icon: RotateCcw,
                   variant: 'default',
                   run: handleRewind,
                 }] as ActionConfig[])
               : []),
            {
              key: 'run',
              label: 'Run',
              icon: Play,
              variant: 'default',
              run: () => store.runPipeline(),
            } as ActionConfig,
            {
              key: 'delete',
              label: 'Delete',
              icon: Trash2,
              variant: 'destructive',
              run: handleDelete,
            } as ActionConfig,
          ];
    case 'cancelled':
      return [
        {
          key: 'rewind',
          label: 'Rewind',
          icon: RotateCcw,
          variant: 'default',
          run: handleRewind,
        },
        {
          key: 'delete',
          label: 'Delete',
          icon: Trash2,
          variant: 'destructive',
          run: handleDelete,
        },
      ];
    default:
      return [];
  }
});

async function runAction(action: ActionConfig): Promise<void> {
  busyAction.value = action.key;
  try {
    await action.run();
  } finally {
    busyAction.value = null;
  }
}

async function handleCreate(details: PipelineCreateFormSubmit): Promise<void> {
  busyAction.value = 'create';
  try {
    await store.createPipeline(undefined, details);
  } finally {
    busyAction.value = null;
  }
}

async function handleDelete(): Promise<void | undefined> {
  const confirmed = await confirmDialogStore.confirm(
    'Delete this pipeline?',
    'This action cannot be undone.',
  );
  if (!confirmed) {
    return Promise.resolve(undefined);
  }

  return store.deletePipeline();
}

function handleRewind(): Promise<void | undefined> {
  if (rewindableNodeIds.value.length === 0) {
    return Promise.resolve(undefined);
  }

  return store.rewindNodes(rewindableNodeIds.value);
}
</script>

<template>
  <div v-if="showCreateForm || actions.length > 0" class="ml-1 flex flex-wrap items-center gap-2 rounded-lg border border-border-subtle/80 bg-surface-container p-2 shadow-fluent-16" role="group" aria-label="Pipeline quick actions">
    <PipelineCreateForm
      v-if="showCreateForm"
      :disabled="busyAction !== null"
      @submit="handleCreate"
    />
    <Button
      v-for="action in actions"
      v-else
      :key="action.key"
      type="button"
      :variant="action.variant"
      :class="['inline-flex items-center gap-[0.4rem]', action.class]"
      :disabled="busyAction !== null"
      @click="runAction(action)"
    >
      <component :is="action.icon" :size="16" aria-hidden="true" />
      <span>{{ action.label }}</span>
    </Button>
  </div>
</template>
