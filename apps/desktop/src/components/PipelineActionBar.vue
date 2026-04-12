<script setup lang="ts">
import { computed, ref, type Component } from 'vue';
import { storeToRefs } from 'pinia';
import { Pause, Play, RotateCcw, Trash2, XCircle } from 'lucide-vue-next';
import { usePipelineStore } from '../stores/pipeline';

type ActionKey = 'create' | 'run' | 'pause' | 'resume' | 'cancel' | 'delete';

interface ActionConfig {
  key: ActionKey;
  label: string;
  icon: Component;
  className: string;
  run: () => Promise<unknown>;
}

const store = usePipelineStore();
const { currentView, designMode, isRunning, localDefinition } = storeToRefs(store);
const busyAction = ref<ActionKey | null>(null);

const hasDraftNodes = computed(() => (localDefinition.value?.nodes?.length ?? 0) > 0);
const currentStatus = computed(() => currentView.value?.status ?? null);

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

  if (isRunning.value) {
    return [
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
    ];
  }

  switch (currentStatus.value) {
    case 'paused':
      return [
        {
          key: 'resume',
          label: 'Resume',
          icon: RotateCcw,
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
      return [{
        key: 'delete',
        label: 'Delete',
        icon: Trash2,
        className: 'btn-danger',
        run: handleDelete,
      }];
    case 'stuck':
      return [
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
      return [{
        key: 'run',
        label: 'Run',
        icon: Play,
        className: 'btn-primary',
        run: () => store.runPipeline(),
      }];
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
</script>

<template>
  <div v-if="actions.length > 0" class="pipeline-action-bar" role="toolbar" aria-label="Pipeline actions">
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
  border: 1px solid color-mix(in srgb, var(--border-default) 82%, transparent);
  border-radius: 0.85rem;
  background: color-mix(in srgb, var(--bg-main) 82%, transparent);
  backdrop-filter: blur(14px);
  -webkit-backdrop-filter: blur(14px);
  box-shadow: 0 0.5rem 1.5rem color-mix(in srgb, var(--text-primary) 12%, transparent);
}

.pipeline-action-bar__button {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
}
</style>
