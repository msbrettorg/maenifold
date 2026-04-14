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
import SemanticZoomOverlayContent from './SemanticZoomOverlayContent.vue';
import SemanticZoomOverlayControls from './SemanticZoomOverlayControls.vue';
import SemanticZoomOverlayHeader from './SemanticZoomOverlayHeader.vue';

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
  return pipeline.currentView.nodes.find((node) => node.nodeId === zoom.focusedNodeId);
});
const selectedNodeId = computed<string | null>(() => viewNode.value?.nodeId ?? null);
const selectedViewNode = computed<ViewNode | null>(() => viewNode.value ?? null);

const nodeType = computed(() => viewNode.value?.type ?? 'workflow');
const statusLabel = computed(() => viewNode.value?.status ?? 'pending');
const displayLabel = computed(() => (viewNode.value ? getViewNodeDisplayLabel(viewNode.value) : ''));

const assembledPrompt = computed(() => {
  const node = viewNode.value;
  if (!node) return null;
  return 'assembled_prompt' in node ? node.assembled_prompt ?? null : null;
});

const thinkingTrace = computed(() => {
  const node = viewNode.value;
  if (!node || node.type !== 'sequentialThinking') return null;
  return node.thinking_trace ?? null;
});

const workflowSteps = computed(() => {
  const node = viewNode.value;
  if (!node || (node.type !== undefined && node.type !== 'workflow')) return null;
  return 'workflow_steps' in node ? node.workflow_steps ?? null : null;
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

const existingNodeIds = computed(() => currentView.value?.nodes.map((node) => node.nodeId) ?? []);

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
  if (currentView.value) {
    showGraftDialog.value = true;
  }
}

async function handleGraftSubmit(nodes: PipelineNode[], edges: PipelineEdge[]): Promise<void> {
  if (!currentView.value?.pipelineId || graftSubmitting.value) return;

  graftSubmitting.value = true;
  const pipelineId = currentView.value.pipelineId;

  try {
    await pipeline.graftNodes(nodes, edges);
    if (pipeline.errorMessage) return;
    await pipeline.loadPipeline(pipelineId);
    if (!pipeline.errorMessage) {
      showGraftDialog.value = false;
    }
  } finally {
    graftSubmitting.value = false;
  }
}

function openPromotionDialog(): void {
  if (canPromote.value && !promotionBusy.value) {
    showPromotionDialog.value = true;
  }
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

function handleKeyDown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
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
        <SemanticZoomOverlayHeader
          :display-label="displayLabel"
          :node-type="nodeType"
          :status-label="statusLabel"
          @close="zoom.back()"
        />

        <div class="zoom-body">
          <SemanticZoomOverlayContent
            :assembled-prompt="assembledPrompt"
            :node-output="nodeOutput"
            :thinking-trace="thinkingTrace"
            :tool-provenance="toolProvenance"
            :workflow-steps="workflowSteps"
          />
        </div>

        <SemanticZoomOverlayControls
          :busy-action="busyAction"
          :can-promote="canPromote"
          :forked-session-id="forkedSessionId"
          :graft-submitting="graftSubmitting"
          :has-results="hasResults"
          :promotion-busy="promotionBusy"
          :view-node-status="viewNode.status"
          @abandon="abandonNode"
          @fork="forkNode"
          @graft="openGraftDialog"
          @promote="openPromotionDialog"
          @restart="restartNode"
          @rewind="rewindNode"
        />

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
  position: fixed;
  inset: 0;
  z-index: 1000;
  display: grid;
  place-items: center;
  background: rgba(0, 0, 0, 0.6);
}

.zoom-panel {
  width: min(720px, 90vw);
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  background: var(--card, #201f1f);
  border-radius: var(--radius-xl);
  overflow: hidden;
  border: 1px solid var(--border);
  box-shadow: 0 20px 48px rgba(0, 0, 0, 0.35);
}

.zoom-body {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-4);
}
</style>
