<script setup lang="ts">
import { computed, ref, type Component } from 'vue';
import { storeToRefs } from 'pinia';
import { Pause, Play, RotateCcw, Trash2, XCircle } from 'lucide-vue-next';
import type { PipelineStatus } from '../lib/pipeline-types';
import { usePipelineStore } from '../stores/pipeline';

type ActionKey = 'create' | 'run' | 'pause' | 'resume' | 'cancel' | 'rewind' | 'delete';

interface ActionConfig {
  key: ActionKey;
  label: string;
  icon: Component;
  className: string;
  run: () => Promise<unknown>;
}

type PipelineActionStatus = PipelineStatus | 'abandoned';

const store = usePipelineStore();
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

const actions = computed<ActionConfig[]>(() => {
  if (designMode.value) {
    return hasDraftNodes.value
      ? [{
          key: 'create',
          label: 'Create',
          icon: Play,
          className: 'btn-primary',
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
          className: 'btn-primary',
          run: () => store.resumePipeline(),
        },
        {
          key: 'cancel',
          label: 'Cancel',
          icon: XCircle,
          className: 'btn-danger',
          run: () => store.cancelPipeline(),
        },
      ];
    case 'completed':
      return [
        {
          key: 'rewind',
          label: 'Rewind',
          icon: RotateCcw,
          className: 'btn-primary',
          run: handleRewind,
        },
        {
          key: 'delete',
          label: 'Delete',
          icon: Trash2,
          className: 'btn-danger',
          run: handleDelete,
        },
      ];
    case 'abandoned':
      return [
        {
          key: 'rewind',
          label: 'Rewind',
          icon: RotateCcw,
          className: 'btn-primary',
          run: handleRewind,
        },
        {
          key: 'delete',
          label: 'Delete',
          icon: Trash2,
          className: 'btn-danger',
          run: handleDelete,
        },
      ];
    case 'stuck':
      return [
        {
          key: 'rewind',
          label: 'Rewind',
          icon: RotateCcw,
          className: 'btn-primary',
          run: handleRewind,
        },
        {
          key: 'cancel',
          label: 'Cancel',
          icon: XCircle,
          className: 'btn-danger',
          run: () => store.cancelPipeline(),
        },
        {
          key: 'delete',
          label: 'Delete',
          icon: Trash2,
          className: 'btn-danger',
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
              className: 'btn-pause',
              run: () => store.pausePipeline(),
            },
            {
              key: 'cancel',
              label: 'Cancel',
              icon: XCircle,
              className: 'btn-danger',
              run: () => store.cancelPipeline(),
            },
          ]
        : [
            ...(rewindableNodeIds.value.length > 0
              ? [{
                  key: 'rewind' as ActionKey,
                  label: 'Rewind',
                  icon: RotateCcw,
                  className: 'btn-primary',
                  run: handleRewind,
                }]
              : []),
            {
              key: 'run',
              label: 'Run',
              icon: Play,
              className: 'btn-primary',
              run: () => store.runPipeline(),
            },
            {
              key: 'delete',
              label: 'Delete',
              icon: Trash2,
              className: 'btn-danger',
              run: handleDelete,
            },
          ];
    case 'cancelled':
      return [
        {
          key: 'rewind',
          label: 'Rewind',
          icon: RotateCcw,
          className: 'btn-primary',
          run: handleRewind,
        },
        {
          key: 'delete',
          label: 'Delete',
          icon: Trash2,
          className: 'btn-danger',
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

function handleDelete(): Promise<void | undefined> {
  if (!window.confirm('Delete this pipeline? This action cannot be undone.')) {
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
  <div v-if="actions.length > 0" class="pipeline-action-bar" role="group" aria-label="Pipeline quick actions">
    <button
      v-for="action in actions"
      :key="action.key"
      type="button"
      :class="['btn', action.className, 'pipeline-action-bar__button']"
      :disabled="busyAction !== null"
      @click="runAction(action)"
    >
      <component :is="action.icon" :size="16" aria-hidden="true" />
      <span>{{ action.label }}</span>
    </button>
  </div>
</template>

<style scoped>
.pipeline-action-bar {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.5rem;
  margin-left: 0.25rem;
  padding: 0.5rem;
  border: 1px solid color-mix(in srgb, var(--border) 82%, transparent);
  border-radius: 0.85rem;
  background: color-mix(in srgb, var(--background) 82%, transparent);
  backdrop-filter: blur(14px);
  -webkit-backdrop-filter: blur(14px);
  box-shadow: 0 0.5rem 1.5rem color-mix(in srgb, var(--foreground) 12%, transparent);
}

.pipeline-action-bar__button {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
}
</style>
