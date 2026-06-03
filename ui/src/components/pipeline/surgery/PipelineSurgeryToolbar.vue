<script setup lang="ts">
// Surgery toolbar (T-600.5). Teleported into `#pipeline-toolbar`. Surfaces the
// pipeline-level surgery operations — Join (un-buried per spec), Graft, Fork —
// on the selected LIVE pipeline. SUPPRESSED entirely while design mode is active
// (design-mode editing is the parallel system). Per-op enablement comes from the
// verified backend-guard matrix in lib/surgeryEnablement.ts.
import { computed } from 'vue';
import { storeToRefs } from 'pinia';
import { Merge, Scissors, GitFork } from '@lucide/vue';
import { usePipelinesStore } from '../../../stores/pipelines';
import { usePipelineDesignStore } from '../../../stores/pipelineDesign';
import { useSurgeryController } from './useSurgeryController';
import { canGraft, canJoin } from '../../../lib/surgeryEnablement';

const store = usePipelinesStore();
const design = usePipelineDesignStore();
const { selectedView } = storeToRefs(store);
const controller = useSurgeryController();

// Surgery acts on a LIVE, server-backed pipeline only, and never while the
// parallel design-mode editor is open.
const visible = computed(() => !design.designModeActive && !!selectedView.value);

const joinState = computed(() => canJoin(selectedView.value));
const graftState = computed(() => canGraft(selectedView.value));
// Fork has no backend status guard (handler_surgery_fork.rs): always available
// once a pipeline with nodes is selected.
const forkEnabled = computed(() => (selectedView.value?.nodes.length ?? 0) > 0);
</script>

<template>
  <Teleport defer to="#pipeline-toolbar">
    <div v-if="visible" class="surg-tb" role="group" aria-label="Graph surgery">
      <button
        type="button"
        class="surg-btn"
        :disabled="!joinState.enabled"
        :title="joinState.reason ?? 'Merge another pipeline into this one'"
        @click="controller.openDialog('join')"
      >
        <Merge :size="14" :stroke-width="2" /> Join
      </button>
      <button
        type="button"
        class="surg-btn"
        :disabled="!graftState.enabled"
        :title="graftState.reason ?? 'Graft new nodes into this pipeline'"
        @click="controller.openDialog('graft')"
      >
        <Scissors :size="14" :stroke-width="2" /> Graft
      </button>
      <button
        type="button"
        class="surg-btn"
        :disabled="!forkEnabled"
        :title="forkEnabled ? 'Fork a new pipeline from a node' : 'No nodes to fork'"
        @click="controller.openDialog('fork')"
      >
        <GitFork :size="14" :stroke-width="2" /> Fork
      </button>
    </div>
  </Teleport>
</template>

<style scoped>
.surg-tb {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
}

.surg-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  padding: 0.32rem 0.6rem;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary);
  font: inherit;
  font-size: 0.78rem;
  cursor: pointer;
}

.surg-btn:hover:not(:disabled) {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.surg-btn:disabled {
  cursor: default;
  opacity: 0.45;
}
</style>
