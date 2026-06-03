<script setup lang="ts">
// Design-mode toolbar group (T-600.4): the enter/exit toggle, the save-status
// badge (Unsaved / Saving / Saved / Error), and the explicit Save button.
// Teleported into the shell's `#pipeline-toolbar` mount target so it floats over
// the canvas alongside the layout toolbar. All state flows through the design
// store; nothing crosses the teleport boundary via props/emits.
import { computed } from 'vue';
import { storeToRefs } from 'pinia';
import { PencilRuler, Save, X } from '@lucide/vue';
import { usePipelinesStore } from '../../../stores/pipelines';
import { usePipelineDesignStore, type SaveStatus } from '../../../stores/pipelineDesign';

const store = usePipelinesStore();
const { selectedView } = storeToRefs(store);
const design = usePipelineDesignStore();

const active = computed(() => design.designModeActive);
const canEnter = computed(() => Boolean(selectedView.value));

const badge = computed<{ label: string; tone: SaveStatus } | null>(() => {
  if (!active.value) return null;
  const status = design.saveStatus;
  const labels: Record<SaveStatus, string> = {
    idle: 'Ready',
    unsaved: 'Unsaved',
    saving: 'Saving…',
    saved: 'Saved',
    error: 'Error',
  };
  return { label: labels[status], tone: status };
});

const issueCount = computed(() => design.validationIssues.length);

function toggleDesign(): void {
  if (active.value) {
    if (design.confirmDiscardDesignChanges()) design.exitDesignMode();
    return;
  }
  if (canEnter.value) design.enterDesignMode();
}

function save(): void {
  void design.save();
}
</script>

<template>
  <Teleport defer to="#pipeline-toolbar">
    <div class="design-toolbar" role="group" aria-label="Pipeline design tools">
      <button
        type="button"
        class="dt-btn"
        :class="{ active }"
        :disabled="!active && !canEnter"
        :aria-pressed="active"
        :title="active ? 'Exit design mode' : 'Enter design mode'"
        @click="toggleDesign"
      >
        <component :is="active ? X : PencilRuler" :size="15" :stroke-width="2" />
        <span>{{ active ? 'Exit' : 'Design' }}</span>
      </button>

      <template v-if="active">
        <span v-if="badge" class="dt-badge" :class="`tone-${badge.tone}`" role="status">
          {{ badge.label }}
        </span>

        <span
          v-if="issueCount > 0"
          class="dt-issues"
          :title="`${issueCount} validation issue(s) block save`"
        >
          ⚠ {{ issueCount }}
        </span>

        <button
          type="button"
          class="dt-btn dt-save"
          :disabled="!design.canSave"
          title="Save pipeline (update)"
          @click="save"
        >
          <Save :size="15" :stroke-width="2" />
          <span>Save</span>
        </button>
      </template>
    </div>
  </Teleport>
</template>

<style scoped>
.design-toolbar {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  margin-left: 0.4rem;
  padding: 0.3rem 0.4rem;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: color-mix(in srgb, var(--bg-main) 90%, var(--bg-sidebar));
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.dt-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  padding: 0.32rem 0.5rem;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary);
  font: inherit;
  font-size: 0.78rem;
  cursor: pointer;
  transition: background 0.12s ease, color 0.12s ease, border-color 0.12s ease;
}

.dt-btn:hover:not(:disabled) {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.dt-btn.active {
  color: var(--text-accent);
  background: color-mix(in srgb, var(--text-accent) 12%, transparent);
  border-color: color-mix(in srgb, var(--text-accent) 35%, transparent);
}

.dt-btn:disabled {
  cursor: default;
  opacity: 0.45;
}

.dt-save {
  color: var(--text-primary);
  border-color: var(--border-color);
}

.dt-save:not(:disabled) {
  color: var(--node-status-completed, #6cc070);
  border-color: color-mix(in srgb, var(--node-status-completed, #6cc070) 45%, transparent);
}

.dt-badge {
  padding: 0.15rem 0.45rem;
  border-radius: 999px;
  font-size: 0.7rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  border: 1px solid var(--border-color);
  color: var(--text-secondary);
}

.dt-badge.tone-unsaved {
  color: var(--node-status-active, #e0af68);
  border-color: color-mix(in srgb, var(--node-status-active, #e0af68) 45%, transparent);
}

.dt-badge.tone-saving {
  color: var(--text-accent);
  border-color: color-mix(in srgb, var(--text-accent) 45%, transparent);
}

.dt-badge.tone-saved {
  color: var(--node-status-completed, #6cc070);
  border-color: color-mix(in srgb, var(--node-status-completed, #6cc070) 45%, transparent);
}

.dt-badge.tone-error {
  color: var(--bg-danger, #d9534f);
  border-color: color-mix(in srgb, var(--bg-danger, #d9534f) 55%, transparent);
}

.dt-issues {
  font-size: 0.72rem;
  color: var(--node-status-active, #e0af68);
}
</style>
