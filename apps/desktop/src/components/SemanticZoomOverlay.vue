<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { useSemanticZoomStore } from '../stores/semantic-zoom';
import { usePipelineStore } from '../stores/pipeline';
import type { PipelineEdge, PipelineNode, ViewNode } from '../lib/pipeline-types';
import { getViewNodeDisplayLabel } from '../lib/pipeline-types';
import { useInspectorActions } from '../composables/useInspectorActions';
import GraftNodesDialog from './GraftNodesDialog.vue';
import PromotionDialog from './PromotionDialog.vue';

const zoom = useSemanticZoomStore();
const pipeline = usePipelineStore();
const { currentView, workflows } = storeToRefs(pipeline);
const conclusionText = ref('');
const forkedSessionId = ref<string | null>(null);
const showGraftDialog = ref(false);
const graftSubmitting = ref(false);
const showPromotionDialog = ref(false);
const promotionBusy = ref(false);

const viewNode = computed<ViewNode | undefined>(() => {
  if (!zoom.focusedNodeId || !pipeline.currentView) return undefined;
  return pipeline.currentView.nodes.find((n) => n.nodeId === zoom.focusedNodeId);
});
const selectedNodeId = computed<string | null>(() => viewNode.value?.nodeId ?? null);
const selectedViewNode = computed<ViewNode | null>(() => viewNode.value ?? null);

const nodeType = computed(() => viewNode.value?.type ?? 'workflow');
const statusLabel = computed(() => viewNode.value?.status ?? 'pending');
const displayLabel = computed(() => (viewNode.value ? getViewNodeDisplayLabel(viewNode.value) : ''));

const assembledPrompt = computed(() => {
  const node = viewNode.value;
  if (!node) return null;
  if ('assembled_prompt' in node) return node.assembled_prompt ?? null;
  return null;
});

const thinkingTrace = computed(() => {
  const node = viewNode.value;
  if (!node || node.type !== 'sequentialThinking') return null;
  return node.thinking_trace ?? null;
});

const workflowSteps = computed(() => {
  const node = viewNode.value;
  if (!node || (node.type !== undefined && node.type !== 'workflow')) return null;
  if ('workflow_steps' in node) return node.workflow_steps ?? null;
  return null;
});

const nodeOutput = computed(() => viewNode.value?.output ?? null);
const toolProvenance = computed(() => {
  const node = viewNode.value;
  if (!node || node.type !== 'tool') return null;
  return node.toolProvenance ?? null;
});
const { busyAction, runBusyAction, restartNode, rewindNode, abandonNode } = useInspectorActions({
  store: pipeline,
  selectedNodeId,
  selectedViewNode,
  conclusionText,
});

const existingNodeIds = computed(() => currentView.value?.nodes.map((n) => n.nodeId) ?? []);

const hasResults = computed(() => {
  const node = viewNode.value;
  return node?.status === 'completed' && typeof node.output === 'string' && node.output.trim().length > 0;
});

const canPromote = computed(() => (
  hasResults.value
  && typeof currentView.value?.pipelineId === 'string'
  && currentView.value.pipelineId.length > 0
));

function openGraftDialog(): void {
  if (!currentView.value) return;
  showGraftDialog.value = true;
}

async function handleGraftSubmit(nodes: PipelineNode[], edges: PipelineEdge[]): Promise<void> {
  if (!currentView.value?.pipelineId || graftSubmitting.value) return;

  graftSubmitting.value = true;
  const pipelineId = currentView.value.pipelineId;

  try {
    await pipeline.graftNodes(nodes, edges);
    if (pipeline.errorMessage) return;
    await pipeline.loadPipeline(pipelineId);
    if (pipeline.errorMessage) return;
    showGraftDialog.value = false;
  } finally {
    graftSubmitting.value = false;
  }
}

function openPromotionDialog(): void {
  if (!canPromote.value || promotionBusy.value) return;
  showPromotionDialog.value = true;
}

function formatJson(value: Record<string, unknown> | null | undefined): string {
  return value ? JSON.stringify(value, null, 2) : '—';
}

function extractForkedSessionId(result: unknown): string | null {
  if (typeof result === 'string') {
    return result.trim() || null;
  }

  if (!result || typeof result !== 'object') {
    return null;
  }

  const record = result as Record<string, unknown>;
  for (const key of ['sessionId', 'pipelineId', 'newSessionId', 'forkedSessionId']) {
    const value = record[key];
    if (typeof value === 'string' && value.trim().length > 0) {
      return value;
    }
  }

  return null;
}

async function forkNode(): Promise<void> {
  if (!viewNode.value?.sessionId || !viewNode.value.nodeId) return;

  const requestedSessionIdInput = window.prompt('Optional name for the forked pipeline session:', '');
  if (requestedSessionIdInput === null) return;
  const requestedSessionId = requestedSessionIdInput.trim();

  forkedSessionId.value = null;

  const result = await runBusyAction('fork', () =>
    pipeline.forkPipeline(
      viewNode.value!.sessionId!,
      viewNode.value!.nodeId,
      requestedSessionId || undefined,
    ),
  );

  forkedSessionId.value = extractForkedSessionId(result);
}

function handleKeyDown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    zoom.back();
  }
}

watch(() => viewNode.value?.nodeId, () => {
  forkedSessionId.value = null;
  showGraftDialog.value = false;
  showPromotionDialog.value = false;
  promotionBusy.value = false;
});

onMounted(() => document.addEventListener('keydown', handleKeyDown));
onUnmounted(() => document.removeEventListener('keydown', handleKeyDown));
</script>

<template>
  <Teleport to="body">
    <div v-if="zoom.isInspecting && viewNode" class="zoom-overlay" @click.self="zoom.back()">
      <div class="zoom-panel">
        <header class="zoom-header">
          <div class="zoom-header-left">
            <span class="zoom-node-id">{{ displayLabel }}</span>
            <span :class="['zoom-type-pill', `zoom-type--${nodeType}`]">{{ nodeType }}</span>
            <span :class="['zoom-status', `zoom-status--${statusLabel}`]">{{ statusLabel }}</span>
          </div>
          <button class="zoom-close" @click="zoom.back()" aria-label="Close inspection" title="Close (Esc)">
            ×
          </button>
        </header>

        <div class="zoom-body">
          <details v-if="assembledPrompt" class="zoom-section">
            <summary class="zoom-section-title">Assembled Prompt</summary>
            <pre class="zoom-pre">{{ assembledPrompt }}</pre>
          </details>

          <details v-if="thinkingTrace && thinkingTrace.length > 0" class="zoom-section" open>
            <summary class="zoom-section-title">Thinking Trace ({{ thinkingTrace.length }} thoughts)</summary>
            <ol class="zoom-list">
              <li v-for="thought in thinkingTrace" :key="thought.thought_number" class="zoom-list-item">
                <span class="item-label">#{{ thought.thought_number }}</span>
                <span :class="['item-status', `zoom-status--${thought.status}`]">{{ thought.status }}</span>
                <pre class="zoom-pre">{{ thought.text }}</pre>
              </li>
            </ol>
          </details>

          <details v-if="workflowSteps && workflowSteps.length > 0" class="zoom-section" open>
            <summary class="zoom-section-title">Workflow Steps ({{ workflowSteps.length }})</summary>
            <ol class="zoom-list">
              <li v-for="step in workflowSteps" :key="step.step_name" class="zoom-list-item">
                <span class="item-label">{{ step.step_name }}</span>
                <span :class="['item-status', `zoom-status--${step.status}`]">{{ step.status }}</span>
                <pre v-if="step.output" class="zoom-pre">{{ step.output }}</pre>
              </li>
            </ol>
          </details>

          <details v-if="nodeOutput" class="zoom-section" open>
            <summary class="zoom-section-title">Output</summary>
            <pre class="zoom-pre">{{ nodeOutput }}</pre>
          </details>

          <details v-if="toolProvenance" class="zoom-section" open>
            <summary class="zoom-section-title">Tool Provenance</summary>
            <div class="zoom-metadata">
              <span class="item-label">Tool</span>
              <strong>{{ toolProvenance.toolName }}</strong>
            </div>
            <pre class="zoom-pre">{{ formatJson(toolProvenance.resolvedArgs ?? null) }}</pre>
            <pre v-if="toolProvenance.responseSummary" class="zoom-pre">{{ toolProvenance.responseSummary }}</pre>
          </details>

          <div v-if="!assembledPrompt && !thinkingTrace && !workflowSteps && !nodeOutput && !toolProvenance" class="zoom-empty">
            This node hasn't produced any output yet.
          </div>
        </div>

        <footer class="zoom-footer">
          <div v-if="pipeline.errorMessage" class="zoom-error-banner" role="alert">
            <span class="zoom-error-banner__message">{{ pipeline.errorMessage }}</span>
            <button
              class="btn btn-ghost zoom-error-banner__dismiss"
              type="button"
              aria-label="Dismiss error"
              @click="pipeline.clearError()"
            >
              ✕
            </button>
          </div>

          <div class="zoom-actions">
            <button
              :class="['btn', 'btn-ghost', { 'btn--busy': busyAction === 'fork' }]"
              :disabled="busyAction !== null"
              type="button"
              @click="forkNode"
            >
              {{ busyAction === 'fork' ? 'Forking...' : 'Fork from node' }}
            </button>
            <button
              v-if="['abandoned', 'stuck'].includes(viewNode.status)"
              :class="['btn', 'btn-primary', { 'btn--busy': busyAction === 'restart' }]"
              :disabled="busyAction !== null"
              type="button"
              @click="restartNode"
            >
              {{ busyAction === 'restart' ? 'Restarting...' : 'Restart node' }}
            </button>
            <button
              v-if="['completed', 'abandoned', 'stuck'].includes(viewNode.status)"
              :class="['btn', 'btn-ghost', { 'btn--busy': busyAction === 'rewind' }]"
              :disabled="busyAction !== null"
              type="button"
              @click="rewindNode"
            >
              {{ busyAction === 'rewind' ? 'Rewinding...' : 'Rewind node' }}
            </button>
            <button
              v-if="viewNode.status === 'active'"
              :class="['btn', 'btn-danger', { 'btn--busy': busyAction === 'abandon' }]"
              :disabled="busyAction !== null"
              type="button"
              @click="abandonNode"
            >
              {{ busyAction === 'abandon' ? 'Abandoning...' : 'Abandon node' }}
            </button>
            <button
              v-if="hasResults"
              :class="['btn', 'btn-ghost']"
              :disabled="busyAction !== null || graftSubmitting"
              type="button"
              @click="openGraftDialog"
            >
              Graft from node
            </button>
            <button
              v-if="canPromote"
              :class="['btn', 'btn-primary', { 'btn--busy': promotionBusy }]"
              :disabled="promotionBusy || busyAction !== null"
              :title="promotionBusy ? 'Promotion in progress…' : 'Save this completed output to memory'"
              type="button"
              @click="openPromotionDialog"
            >
              {{ promotionBusy ? 'Promoting…' : 'Promote to Memory' }}
            </button>
          </div>

          <div v-if="forkedSessionId" class="zoom-fork-result" role="status">
            <span>Forked session</span>
            <strong class="zoom-mono">{{ forkedSessionId }}</strong>
          </div>
        </footer>

        <GraftNodesDialog
          v-if="showGraftDialog && currentView"
          :busy="graftSubmitting"
          :existing-node-ids="existingNodeIds"
          :workflows="workflows"
          @cancel="showGraftDialog = false"
          @submit="handleGraftSubmit"
        />
        <PromotionDialog
          v-if="showPromotionDialog && canPromote && currentView?.pipelineId"
          :node="viewNode!"
          :pipeline-id="currentView.pipelineId"
          @busy-change="promotionBusy = $event"
          @close="showPromotionDialog = false"
        />
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.zoom-overlay {
  position: fixed; inset: 0; z-index: 1000;
  display: grid; place-items: center;
  background: rgba(0, 0, 0, 0.6);
}
.zoom-panel {
  width: min(720px, 90vw); max-height: 80vh;
  display: flex; flex-direction: column;
  background: var(--surface-container, #201f1f);
  border-radius: var(--radius-xl); overflow: hidden;
  border: 1px solid var(--border-default);
  box-shadow: 0 20px 48px rgba(0, 0, 0, 0.35);
}
.zoom-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: var(--space-3) var(--space-4);
  background: var(--surface-container-highest, #353534);
}
.zoom-header-left { display: flex; align-items: center; gap: var(--space-2); }
.zoom-node-id { font-family: var(--font-mono); font-size: var(--text-sm); font-weight: 600; color: var(--text-primary); }
.zoom-type-pill {
  font-size: var(--text-xs); padding: 2px 8px; border-radius: 999px;
  color: var(--text-primary);
  background: color-mix(in srgb, var(--color-node-workflow) 15%, transparent);
}
.zoom-type--prompt { background: color-mix(in srgb, var(--color-node-prompt) 15%, transparent); }
.zoom-type--sequentialThinking { background: color-mix(in srgb, var(--color-node-thinking) 15%, transparent); }
.zoom-type--tool { background: color-mix(in srgb, var(--color-node-tool) 15%, transparent); }
.zoom-status {
  font-size: var(--text-xs); padding: 2px 6px; border-radius: 999px;
  color: var(--text-secondary);
  background: color-mix(in srgb, var(--color-status-pending) 15%, transparent);
}
.zoom-status--active { background: color-mix(in srgb, var(--color-status-active) 15%, transparent); color: var(--color-status-active); }
.zoom-status--completed { background: color-mix(in srgb, var(--color-status-completed) 15%, transparent); color: var(--color-status-completed); }
.zoom-status--abandoned { background: color-mix(in srgb, var(--color-status-abandoned) 15%, transparent); color: var(--color-status-abandoned); }
.zoom-status--ready { background: color-mix(in srgb, var(--color-status-ready) 15%, transparent); color: var(--color-status-ready); }
.zoom-close {
  background: none; border: none; color: var(--text-muted);
  font-size: var(--text-xl); cursor: pointer; padding: var(--space-1);
  line-height: 1; border-radius: var(--radius-sm);
}
.zoom-close:hover { color: var(--text-primary); background: var(--bg-surface); }
.zoom-close:focus-visible { outline: 2px solid var(--border-focus); outline-offset: 2px; }
.zoom-body {
  flex: 1; overflow-y: auto; padding: var(--space-4);
  display: flex; flex-direction: column; gap: var(--space-3);
}
.zoom-footer {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding: var(--space-3) var(--space-4) var(--space-4);
  border-top: 1px solid var(--border-default);
  background: color-mix(in srgb, var(--surface-container-highest, #353534) 55%, var(--surface-container, #201f1f));
}
.zoom-actions { display: flex; flex-wrap: wrap; gap: 0.75rem; }
.zoom-fork-result {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  padding: 0.75rem;
  border: 1px solid var(--border-default);
  border-radius: 8px;
  background: var(--bg-main);
}
.zoom-fork-result span { color: var(--text-secondary); font-size: var(--text-sm); }
.zoom-error-banner {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.5rem 0.75rem;
  background: var(--bg-error);
  color: var(--text-error);
  border: 1px solid var(--border-error);
  border-radius: 6px;
  font-size: var(--text-sm);
}
.zoom-error-banner__message { flex: 1; }
.zoom-error-banner__dismiss { flex-shrink: 0; color: inherit; }
.zoom-error-banner__dismiss:hover { opacity: 0.75; }
.zoom-section-title {
  font-family: var(--font-headline, var(--font-sans));
  font-size: var(--text-sm); font-weight: 600; color: var(--text-primary);
  cursor: pointer; padding: var(--space-2) 0; list-style: none;
}
.zoom-section-title::before { content: '▸ '; }
.zoom-section[open] > .zoom-section-title::before { content: '▾ '; }
.zoom-pre {
  margin: var(--space-2) 0 0; padding: var(--space-3);
  font-family: var(--font-mono); font-size: var(--text-xs); line-height: 1.5;
  color: var(--text-secondary); background: var(--bg-main);
  border-radius: var(--radius-md); white-space: pre-wrap; word-break: break-word;
  max-height: 300px; overflow-y: auto;
}
.zoom-list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: var(--space-2); }
.zoom-list-item {
  display: flex; flex-direction: column; gap: var(--space-1);
  padding: var(--space-2); background: var(--bg-surface); border-radius: var(--radius-md);
}
.zoom-metadata {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  margin-top: var(--space-2);
}
.zoom-mono { font-family: var(--font-mono); word-break: break-all; }
.item-label { font-family: var(--font-mono); font-size: var(--text-xs); font-weight: 600; color: var(--text-primary); }
.item-status { font-size: var(--text-xs); color: var(--text-muted); }
.zoom-empty { padding: var(--space-8); text-align: center; color: var(--text-muted); font-size: var(--text-sm); }
</style>
