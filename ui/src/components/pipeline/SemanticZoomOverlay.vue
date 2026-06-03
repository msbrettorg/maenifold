<script setup lang="ts">
// SemanticZoom overlay shell (T-600.6). ONE renderer that dispatches to five
// content views based on what the selected node carries. Teleported into the
// shell's `#pipeline-overlay` mount target. Opened by the node inspector via the
// shared canvas controller (`controller.zoomNodeId`); a node may expose several
// surfaces (e.g. a sequentialThinking node has prompt + thinking + output), so
// the shell offers a tab per available surface.
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { BookPlus, X } from '@lucide/vue';
import { usePipelinesStore } from '../../stores/pipelines';
import { useCanvasController } from './useCanvasController';
import { usePromoteToMemory } from '../../composables/usePromoteToMemory';
import {
  availableSurfaces,
  type ZoomSurface,
} from './zoomLogic';
import {
  isSequentialThinkingViewNode,
  isToolViewNode,
  isWorkflowViewNode,
  viewNodeDisplayLabel,
  viewNodeType,
  type ViewNode,
} from '../../lib/pipeline-types';
import { isInspectableNode } from '../../lib/sessionResolve';
import ZoomPromptView from './ZoomPromptView.vue';
import ZoomThinkingView from './ZoomThinkingView.vue';
import ZoomWorkflowView from './ZoomWorkflowView.vue';
import ZoomOutputView from './ZoomOutputView.vue';
import ZoomProvenanceView from './ZoomProvenanceView.vue';
import SessionInspectorPanel from './inspect/SessionInspectorPanel.vue';

const store = usePipelinesStore();
const { selectedNodes } = storeToRefs(store);
const controller = useCanvasController();
const promote = usePromoteToMemory();

function promoteCurrentNode(): void {
  if (node.value) promote.openFor(node.value);
}

const node = computed<ViewNode | null>(() => {
  const id = controller.zoomNodeId.value;
  if (!id) return null;
  return selectedNodes.value.find((candidate) => candidate.nodeId === id) ?? null;
});

// The live ACP session is offered first for any inspectable (non-tool) node, so
// opening zoom on a running node lands on its live stream — the attachment is
// owned/driven by NodeInspectorPane's watch on the zoom-or-selected target, so
// opening the overlay auto-reconnects (scope #4). Content surfaces follow.
const surfaces = computed<ZoomSurface[]>(() => {
  const n = node.value;
  if (!n) return [];
  const live: ZoomSurface[] = isInspectableNode(n) ? ['live'] : [];
  return [...live, ...availableSurfaces(n)];
});

const SURFACE_LABELS: Record<ZoomSurface, string> = {
  live: 'Live',
  prompt: 'Prompt',
  thinking: 'Thinking',
  workflow: 'Steps',
  output: 'Output',
  provenance: 'Provenance',
};

const activeSurface = ref<ZoomSurface | null>(null);

// Keep the active tab valid as the node (and therefore its surfaces) changes.
watch(
  surfaces,
  (next) => {
    if (next.length === 0) {
      activeSurface.value = null;
      return;
    }
    if (!activeSurface.value || !next.includes(activeSurface.value)) {
      activeSurface.value = next[0];
    }
  },
  { immediate: true }
);

// Narrowed accessors for the renderers (each guards on the relevant surface).
const assembledPrompt = computed(() => {
  const n = node.value;
  if (!n) return '';
  return (n as { assembledPrompt?: string | null }).assembledPrompt ?? '';
});
const thinkingTrace = computed(() =>
  node.value && isSequentialThinkingViewNode(node.value) ? node.value.thinkingTrace ?? [] : []
);
const workflowSteps = computed(() =>
  node.value && isWorkflowViewNode(node.value) ? node.value.workflowSteps ?? [] : []
);
const output = computed(() => node.value?.output ?? '');
const toolProvenance = computed(() =>
  node.value && isToolViewNode(node.value) ? node.value.toolProvenance ?? null : null
);

const closeButton = ref<HTMLButtonElement | null>(null);
let restoreFocus: HTMLElement | null = null;

function close(): void {
  controller.closeZoom();
}

// Focus management: on open, remember the previously focused element and move
// focus to the close button; on close, restore focus. We do NOT trap focus —
// the user can Tab out freely, and Esc always dismisses.
watch(
  () => controller.zoomNodeId.value,
  (id) => {
    if (id) {
      restoreFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      void nextTick(() => closeButton.value?.focus());
    } else if (restoreFocus) {
      restoreFocus.focus();
      restoreFocus = null;
    }
  }
);

function onKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape' && controller.zoomNodeId.value) {
    event.stopPropagation();
    close();
  }
}

window.addEventListener('keydown', onKeydown);
onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown));
</script>

<template>
  <Teleport defer to="#pipeline-overlay">
    <div
      v-if="node"
      class="zoom-scrim"
      role="dialog"
      aria-modal="true"
      :aria-label="`Semantic zoom: ${viewNodeDisplayLabel(node)}`"
      @click.self="close"
    >
      <div class="zoom-panel">
        <header class="zoom-header">
          <div class="zoom-title">
            <span class="zoom-type" :class="`type-${viewNodeType(node)}`">{{ viewNodeType(node) }}</span>
            <strong>{{ viewNodeDisplayLabel(node) }}</strong>
            <span class="zoom-status">{{ node.status }}</span>
          </div>
          <div class="zoom-actions">
            <button
              type="button"
              class="zoom-promote"
              aria-label="Promote to memory"
              title="Promote to memory"
              @click="promoteCurrentNode"
            >
              <BookPlus :size="15" :stroke-width="2" />
              <span>Promote</span>
            </button>
            <button ref="closeButton" type="button" class="zoom-close" aria-label="Close zoom" @click="close">
              <X :size="16" :stroke-width="2" />
            </button>
          </div>
        </header>

        <nav v-if="surfaces.length > 1" class="zoom-tabs" role="tablist">
          <button
            v-for="surface in surfaces"
            :key="surface"
            type="button"
            class="zoom-tab"
            role="tab"
            :aria-selected="activeSurface === surface"
            :class="{ active: activeSurface === surface }"
            @click="activeSurface = surface"
          >
            {{ SURFACE_LABELS[surface] }}
          </button>
        </nav>

        <div class="zoom-body">
          <p v-if="surfaces.length === 0" class="zoom-empty">
            This node carries no inspectable cognitive surfaces yet.
          </p>
          <SessionInspectorPanel v-else-if="activeSurface === 'live'" :node="node" />
          <ZoomPromptView v-else-if="activeSurface === 'prompt'" :assembled-prompt="assembledPrompt" />
          <ZoomThinkingView v-else-if="activeSurface === 'thinking'" :trace="thinkingTrace" />
          <ZoomWorkflowView v-else-if="activeSurface === 'workflow'" :steps="workflowSteps" />
          <ZoomOutputView v-else-if="activeSurface === 'output'" :output="output" />
          <ZoomProvenanceView
            v-else-if="activeSurface === 'provenance' && toolProvenance"
            :provenance="toolProvenance"
          />
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.zoom-scrim {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1.5rem;
  background: color-mix(in srgb, var(--bg-main) 60%, transparent);
  backdrop-filter: blur(2px);
}

.zoom-panel {
  display: flex;
  flex-direction: column;
  width: min(760px, 100%);
  max-height: 100%;
  border: 1px solid var(--border-color);
  border-radius: 12px;
  background: color-mix(in srgb, var(--bg-main) 96%, var(--bg-sidebar));
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.28);
  overflow: hidden;
}

.zoom-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.7rem 0.85rem;
  border-bottom: 1px solid var(--border-color);
  background: color-mix(in srgb, var(--bg-sidebar) 55%, transparent);
}

.zoom-title {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  min-width: 0;
}

.zoom-title strong {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.92rem;
  color: var(--text-primary);
}

.zoom-type {
  flex-shrink: 0;
  padding: 0.08rem 0.4rem;
  border-radius: 5px;
  font-size: 0.66rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  background: color-mix(in srgb, var(--node-accent-workflow) 18%, transparent);
  color: color-mix(in srgb, var(--node-accent-workflow) 80%, var(--text-primary));
}

.zoom-type.type-prompt {
  background: color-mix(in srgb, var(--node-accent-prompt) 18%, transparent);
  color: color-mix(in srgb, var(--node-accent-prompt) 82%, var(--text-primary));
}
.zoom-type.type-sequentialThinking {
  background: color-mix(in srgb, var(--node-accent-thinking) 18%, transparent);
  color: color-mix(in srgb, var(--node-accent-thinking) 82%, var(--text-primary));
}
.zoom-type.type-tool {
  background: color-mix(in srgb, var(--node-accent-tool) 18%, transparent);
  color: color-mix(in srgb, var(--node-accent-tool) 80%, var(--text-primary));
}

.zoom-status {
  flex-shrink: 0;
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  color: var(--text-secondary);
}

.zoom-actions {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  flex-shrink: 0;
}

.zoom-promote {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  padding: 0.3rem 0.55rem;
  border: 1px solid color-mix(in srgb, var(--text-accent) 35%, transparent);
  border-radius: 7px;
  background: color-mix(in srgb, var(--text-accent) 10%, transparent);
  color: var(--text-accent);
  cursor: pointer;
  font: inherit;
  font-size: 0.76rem;
  font-weight: 600;
  transition: background 0.12s ease, color 0.12s ease;
}

.zoom-promote:hover {
  background: color-mix(in srgb, var(--text-accent) 18%, transparent);
}

.zoom-close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0.3rem;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  transition: background 0.12s ease, color 0.12s ease;
}

.zoom-close:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.zoom-tabs {
  display: flex;
  gap: 0.2rem;
  padding: 0.45rem 0.6rem 0;
  border-bottom: 1px solid var(--border-color);
}

.zoom-tab {
  padding: 0.35rem 0.7rem;
  border: 1px solid transparent;
  border-bottom: none;
  border-radius: 7px 7px 0 0;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 0.78rem;
  font-weight: 600;
  transition: background 0.12s ease, color 0.12s ease;
}

.zoom-tab:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.zoom-tab.active {
  color: var(--text-accent);
  background: color-mix(in srgb, var(--text-accent) 10%, transparent);
  border-color: var(--border-color);
}

.zoom-body {
  flex: 1;
  min-height: 0;
  padding: 0.9rem;
  overflow-y: auto;
}

.zoom-empty {
  margin: 0;
  color: var(--text-muted);
  font-size: 0.84rem;
}
</style>
