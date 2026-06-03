<script setup lang="ts">
// Design-mode context menu (T-600.4). Teleported into `#pipeline-overlay`.
// Right-clicking the canvas pane / a node / an edge opens the matching menu (the
// canvas captures the native events and records the menu state on the design
// controller singleton; this component renders it). All actions mutate the
// design store; positions for new nodes come from the captured flow-space cursor.
import { computed, onBeforeUnmount, watch } from 'vue';
import { CopyPlus, Pencil, Plus, Scissors, Trash2, Unlink } from '@lucide/vue';
import { usePipelineDesignStore } from '../../../stores/pipelineDesign';
import { useDesignController } from './useDesignController';
import { indexForFlowEdgeId } from '../../../lib/pipelineDefinitionProjection';
import { ADD_NODE_LABELS, type AddNodeKind } from '../../../lib/pipelineDesignOps';

const design = usePipelineDesignStore();
const controller = useDesignController();

const menu = controller.menu;
const open = computed(() => menu.value !== null);

const ADD_ITEMS: AddNodeKind[] = [
  'ask-agent',
  'add-session-node',
  'run-tool',
  'think',
  'run-workflow',
];

const menuStyle = computed(() => ({
  left: `${menu.value?.x ?? 0}px`,
  top: `${menu.value?.y ?? 0}px`,
}));

function addNode(kind: AddNodeKind): void {
  const id = design.addNode(kind, menu.value?.flowPos);
  controller.closeMenu();
  if (id) controller.openNodeForm(id);
}

function editNode(): void {
  const id = menu.value?.targetId;
  controller.closeMenu();
  if (id) controller.openNodeForm(id);
}

function duplicateNode(): void {
  const id = menu.value?.targetId;
  if (id) design.duplicateNodeById(id);
  controller.closeMenu();
}

function deleteNode(): void {
  const id = menu.value?.targetId;
  if (id) design.deleteNodeById(id);
  controller.closeMenu();
}

function disconnectNode(): void {
  const id = menu.value?.targetId;
  if (id) design.disconnectNodeEdges(id);
  controller.closeMenu();
}

function resolveEdgeIndex(): number {
  const edges = design.localDefinition?.edges ?? [];
  const flowId = menu.value?.targetId ?? '';
  return indexForFlowEdgeId(edges, flowId);
}

function editEdge(): void {
  const index = resolveEdgeIndex();
  controller.closeMenu();
  if (index >= 0) controller.openEdgeForm(index);
}

function disconnectEdge(): void {
  const index = resolveEdgeIndex();
  if (index >= 0) design.deleteEdgeByIndex(index);
  controller.closeMenu();
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
        <template v-if="menu?.kind === 'pane'">
          <span class="ctx-title">Add node</span>
          <button
            v-for="kind in ADD_ITEMS"
            :key="kind"
            type="button"
            class="ctx-item"
            role="menuitem"
            @click="addNode(kind)"
          >
            <Plus :size="14" :stroke-width="2" />
            {{ ADD_NODE_LABELS[kind] }}
          </button>
        </template>

        <template v-else-if="menu?.kind === 'node'">
          <span class="ctx-title">Node</span>
          <button type="button" class="ctx-item" role="menuitem" @click="editNode">
            <Pencil :size="14" :stroke-width="2" /> Edit
          </button>
          <button type="button" class="ctx-item" role="menuitem" @click="duplicateNode">
            <CopyPlus :size="14" :stroke-width="2" /> Duplicate
          </button>
          <button type="button" class="ctx-item" role="menuitem" @click="disconnectNode">
            <Scissors :size="14" :stroke-width="2" /> Disconnect edges
          </button>
          <button type="button" class="ctx-item danger" role="menuitem" @click="deleteNode">
            <Trash2 :size="14" :stroke-width="2" /> Delete
          </button>
        </template>

        <template v-else-if="menu?.kind === 'edge'">
          <span class="ctx-title">Edge</span>
          <button type="button" class="ctx-item" role="menuitem" @click="editEdge">
            <Pencil :size="14" :stroke-width="2" /> Edit
          </button>
          <button type="button" class="ctx-item danger" role="menuitem" @click="disconnectEdge">
            <Unlink :size="14" :stroke-width="2" /> Disconnect
          </button>
        </template>
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
  min-width: 180px;
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

.ctx-item:hover {
  background: var(--bg-hover);
}

.ctx-item.danger {
  color: var(--bg-danger, #d9534f);
}

.ctx-item.danger:hover {
  background: color-mix(in srgb, var(--bg-danger, #d9534f) 14%, transparent);
}
</style>
