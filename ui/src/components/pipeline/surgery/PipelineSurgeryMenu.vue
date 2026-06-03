<script setup lang="ts">
// Surgery context menu (T-600.5). Teleported into `#pipeline-overlay`. Opens on
// a node right-click while NOT in design mode (the canvas records the target on
// the surgery controller singleton). PARALLEL to the design context menu, never
// shared. Each item is gated by the verified per-op enablement matrix
// (lib/surgeryEnablement) — illegal actions are disabled with an explaining
// tooltip rather than issued optimistically. Actions run through useSurgeryActions
// (pause→mutate→view→refresh→resume), then the menu closes.
import { computed, onBeforeUnmount, watch } from 'vue';
import { GitFork, Octagon, RotateCcw, Scissors, Undo2 } from '@lucide/vue';
import { storeToRefs } from 'pinia';
import { usePipelinesStore } from '../../../stores/pipelines';
import { useCanvasController } from '../useCanvasController';
import { useSurgeryController } from './useSurgeryController';
import { useSurgeryActions } from './useSurgeryActions';
import {
  canAbandonNode,
  canBatchRewind,
  canForkNode,
  canGraft,
  canRestartNode,
  canRewindNode,
} from '../../../lib/surgeryEnablement';
import type { ViewNode } from '../../../lib/pipeline-types';

const store = usePipelinesStore();
const { selectedView, selectedNodes } = storeToRefs(store);
const canvas = useCanvasController();
const controller = useSurgeryController();
const actions = useSurgeryActions();

const menu = controller.menu;
const open = computed(() => menu.value !== null);

const targetNode = computed<ViewNode | null>(() => {
  const id = menu.value?.nodeId;
  if (!id) return null;
  return selectedNodes.value.find((node) => node.nodeId === id) ?? null;
});

// Batch rewind targets: the full multi-selection when the right-clicked node is
// part of it; otherwise just the single right-clicked node.
const rewindIds = computed<string[]>(() => {
  const id = menu.value?.nodeId;
  if (!id) return [];
  const selected = canvas.selectedNodeIds.value;
  if (selected.length > 1 && selected.includes(id)) return [...selected];
  return [id];
});

const menuStyle = computed(() => ({
  left: `${menu.value?.x ?? 0}px`,
  top: `${menu.value?.y ?? 0}px`,
}));

const rewindEnable = computed(() => canBatchRewind(selectedView.value, rewindIds.value));
const restartEnable = computed(() => canRestartNode(selectedView.value, targetNode.value));
const abandonEnable = computed(() => canAbandonNode(targetNode.value));
const forkEnable = computed(() => canForkNode(targetNode.value));
const graftEnable = computed(() => canGraft(selectedView.value));
// Single-node rewind reason (used when only one node is targeted).
const rewindReasonSingle = computed(() => canRewindNode(selectedView.value).reason);

const rewindLabel = computed(() =>
  rewindIds.value.length > 1 ? `Rewind ${rewindIds.value.length} selected nodes` : 'Rewind node'
);

async function doRewind(): Promise<void> {
  const ids = rewindIds.value;
  controller.closeMenu();
  if (ids.length === 0) return;
  await actions.rewind(ids);
}

async function doRestart(): Promise<void> {
  const id = menu.value?.nodeId;
  controller.closeMenu();
  if (id) await actions.restartNode(id);
}

async function doAbandon(): Promise<void> {
  const id = menu.value?.nodeId;
  controller.closeMenu();
  if (id) await actions.abandonNode(id);
}

function openGraft(): void {
  const id = menu.value?.nodeId ?? null;
  controller.openDialog('graft', id);
}

function openFork(): void {
  const id = menu.value?.nodeId ?? null;
  controller.openDialog('fork', id);
}

function onKey(event: KeyboardEvent): void {
  if (event.key === 'Escape') controller.closeMenu();
}

watch(open, (isOpen) => {
  if (typeof window === 'undefined') return;
  if (isOpen) window.addEventListener('keydown', onKey);
  else window.removeEventListener('keydown', onKey);
});

onBeforeUnmount(() => {
  if (typeof window !== 'undefined') window.removeEventListener('keydown', onKey);
});
</script>

<template>
  <Teleport defer to="#pipeline-overlay">
    <div v-if="open" class="ctx-backdrop" @click="controller.closeMenu()" @contextmenu.prevent>
      <div class="ctx-menu" :style="menuStyle" role="menu" @click.stop>
        <span class="ctx-title">Surgery · {{ menu?.nodeId }}</span>

        <button
          type="button"
          class="ctx-item"
          role="menuitem"
          :disabled="!rewindEnable.enabled || actions.busy.value"
          :title="rewindEnable.reason ?? rewindReasonSingle ?? rewindLabel"
          @click="doRewind"
        >
          <Undo2 :size="14" :stroke-width="2" /> {{ rewindLabel }}
        </button>

        <button
          type="button"
          class="ctx-item"
          role="menuitem"
          :disabled="!restartEnable.enabled || actions.busy.value"
          :title="restartEnable.reason ?? 'Restart this node'"
          @click="doRestart"
        >
          <RotateCcw :size="14" :stroke-width="2" /> Restart node
        </button>

        <button
          type="button"
          class="ctx-item"
          role="menuitem"
          :disabled="!abandonEnable.enabled || actions.busy.value"
          :title="abandonEnable.reason ?? 'Abandon this active node'"
          @click="doAbandon"
        >
          <Octagon :size="14" :stroke-width="2" /> Abandon node
        </button>

        <span class="ctx-sep" aria-hidden="true" />

        <button
          type="button"
          class="ctx-item"
          role="menuitem"
          :disabled="!graftEnable.enabled || actions.busy.value"
          :title="graftEnable.reason ?? 'Graft new nodes after this node'"
          @click="openGraft"
        >
          <Scissors :size="14" :stroke-width="2" /> Graft nodes…
        </button>

        <button
          type="button"
          class="ctx-item"
          role="menuitem"
          :disabled="!forkEnable.enabled || actions.busy.value"
          :title="forkEnable.reason ?? 'Fork a new pipeline from this node'"
          @click="openFork"
        >
          <GitFork :size="14" :stroke-width="2" /> Fork from node…
        </button>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.ctx-backdrop {
  position: fixed;
  inset: 0;
  z-index: 30;
}

.ctx-menu {
  position: fixed;
  min-width: 200px;
  display: flex;
  flex-direction: column;
  gap: 0.1rem;
  padding: 0.3rem;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: color-mix(in srgb, var(--bg-main) 96%, var(--bg-sidebar));
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.28);
}

.ctx-title {
  padding: 0.2rem 0.45rem;
  font-size: 0.66rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--text-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ctx-sep {
  height: 1px;
  margin: 0.2rem 0.3rem;
  background: var(--border-color);
}

.ctx-item {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  width: 100%;
  padding: 0.4rem 0.45rem;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-primary);
  font: inherit;
  font-size: 0.8rem;
  text-align: left;
  cursor: pointer;
  transition: background 0.1s ease, color 0.1s ease;
}

.ctx-item:hover:not(:disabled) {
  background: var(--bg-hover);
}

.ctx-item:disabled {
  cursor: default;
  opacity: 0.45;
}
</style>
