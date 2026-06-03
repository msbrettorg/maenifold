<script setup lang="ts">
// Lifecycle action bar for the selected pipeline. Pure presentation: button
// enablement comes from the `flags` prop (derived by `runnerActions.actionFlags`)
// and every click emits a semantic event the container maps to a runner action.
import { Ban, Pause, Play, Plus, Trash2 } from '@lucide/vue';
import type { PipelineStatus } from '../../../lib/pipeline-types';
import type { ActionFlags } from './runnerActions';

defineProps<{
  flags: ActionFlags;
  status: PipelineStatus | null;
  busy: boolean;
}>();

const emit = defineEmits<{
  (event: 'new'): void;
  (event: 'run'): void;
  (event: 'pause'): void;
  (event: 'resume'): void;
  (event: 'cancel'): void;
  (event: 'delete'): void;
}>();
</script>

<template>
  <div class="abar" role="toolbar" aria-label="Pipeline lifecycle actions">
    <span class="abar-status" :class="status ?? 'none'">{{ status ?? 'no selection' }}</span>

    <span class="abar-sep" aria-hidden="true"></span>

    <button type="button" class="abtn" :disabled="busy" title="New pipeline" @click="emit('new')">
      <Plus :size="15" :stroke-width="2" />
      <span>New</span>
    </button>

    <button
      type="button"
      class="abtn primary"
      :disabled="busy || !flags.canRun"
      title="Run pipeline"
      @click="emit('run')"
    >
      <Play :size="15" :stroke-width="2" />
      <span>Run</span>
    </button>

    <button
      v-if="flags.canResume"
      type="button"
      class="abtn"
      :disabled="busy"
      title="Resume pipeline"
      @click="emit('resume')"
    >
      <Play :size="15" :stroke-width="2" />
      <span>Resume</span>
    </button>
    <button
      v-else
      type="button"
      class="abtn"
      :disabled="busy || !flags.canPause"
      title="Pause pipeline"
      @click="emit('pause')"
    >
      <Pause :size="15" :stroke-width="2" />
      <span>Pause</span>
    </button>

    <button
      type="button"
      class="abtn"
      :disabled="busy || !flags.canCancel"
      title="Cancel pipeline"
      @click="emit('cancel')"
    >
      <Ban :size="15" :stroke-width="2" />
      <span>Cancel</span>
    </button>

    <button
      type="button"
      class="abtn danger"
      :disabled="busy || !flags.canDelete"
      title="Delete pipeline"
      @click="emit('delete')"
    >
      <Trash2 :size="15" :stroke-width="2" />
      <span>Delete</span>
    </button>
  </div>
</template>

<style scoped>
.abar {
  display: flex;
  align-items: center;
  gap: 0.3rem;
  flex-wrap: wrap;
}

.abar-status {
  padding: 0.18rem 0.5rem;
  border-radius: 999px;
  font-size: 0.7rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  color: var(--text-secondary);
  background: color-mix(in srgb, var(--text-muted) 18%, transparent);
}

.abar-status.running,
.abar-status.active,
.abar-status.retrying {
  color: var(--node-status-active, var(--text-accent));
  background: color-mix(in srgb, var(--node-status-active, var(--text-accent)) 16%, transparent);
}

.abar-status.completed {
  color: var(--node-status-completed, var(--text-accent));
  background: color-mix(in srgb, var(--node-status-completed, var(--text-accent)) 16%, transparent);
}

.abar-status.paused {
  color: var(--node-status-stuck, var(--text-secondary));
  background: color-mix(in srgb, var(--node-status-stuck, var(--text-secondary)) 16%, transparent);
}

.abar-status.failed,
.abar-status.stuck,
.abar-status.abandoned,
.abar-status.cancelled {
  color: var(--bg-danger);
  background: color-mix(in srgb, var(--bg-danger) 14%, transparent);
}

.abar-sep {
  width: 1px;
  height: 18px;
  margin: 0 0.15rem;
  background: var(--border-color);
}

.abtn {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  padding: 0.32rem 0.55rem;
  border: 1px solid var(--border-color);
  border-radius: 7px;
  background: color-mix(in srgb, var(--bg-main) 96%, var(--bg-sidebar));
  color: var(--text-secondary);
  font: inherit;
  font-size: 0.78rem;
  cursor: pointer;
  transition: background 0.12s ease, color 0.12s ease, border-color 0.12s ease;
}

.abtn:hover:not(:disabled) {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.abtn:disabled {
  cursor: default;
  opacity: 0.45;
}

.abtn.primary {
  color: var(--text-accent);
  border-color: color-mix(in srgb, var(--text-accent) 35%, transparent);
  background: color-mix(in srgb, var(--text-accent) 12%, transparent);
}

.abtn.primary:hover:not(:disabled) {
  background: color-mix(in srgb, var(--text-accent) 20%, transparent);
}

.abtn.danger:hover:not(:disabled) {
  color: var(--bg-danger);
  border-color: color-mix(in srgb, var(--bg-danger) 40%, transparent);
  background: color-mix(in srgb, var(--bg-danger) 12%, transparent);
}
</style>
