<script setup lang="ts">
import { Button } from '@/components/ui/button';
import { computed, ref, type Component } from 'vue';
import { storeToRefs } from 'pinia';
import { Pause, Play, RotateCcw, Trash2, XCircle } from 'lucide-vue-next';
import type { PipelineStatus } from '../lib/pipeline-types';
import { usePipelineStore } from '../stores/pipeline';
import { useConfirmDialogStore } from '../stores/confirm-dialog';

type ActionKey = 'create' | 'run' | 'pause' | 'resume' | 'cancel' | 'rewind' | 'delete';

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
    return hasDraftNodes.value
        ? [{
            key: 'create',
            label: 'Create',
            icon: Play,
            variant: 'default',
            run: () => store.createPipeline(),
          }]
      : [];
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
  <div v-if="actions.length > 0" class="flex flex-wrap items-center gap-2 ml-1 p-2 rounded-[var(--radius)] border border-[color-mix(in_srgb,var(--color-border-subtle)_82%,transparent)] bg-[var(--surface-container)] shadow-[var(--shadow-16)]" role="group" aria-label="Pipeline quick actions">
    <Button
      v-for="action in actions"
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
