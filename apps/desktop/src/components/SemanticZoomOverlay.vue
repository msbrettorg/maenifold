<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { toast } from 'vue-sonner';
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog';
import { useSemanticZoomStore } from '../stores/semantic-zoom';
import { usePipelineStore } from '../stores/pipeline';
import { useSessionStore } from '../stores/session';
import { useAppShellStore } from '../stores/app-shell';
import { usePromptDialogStore } from '../stores/prompt-dialog';
import type { PipelineEdge, PipelineNode, ViewNode } from '../lib/pipeline-types';
import { getViewNodeDisplayLabel } from '../lib/pipeline-types';
import { useInspectorActions } from '../composables/useInspectorActions';
import { latestAssistantMessageForRemoteSession } from '../composables/usePipelineRefinementSession';
import GraftNodesDialog from './GraftNodesDialog.vue';
import PromotionDialog from './PromotionDialog.vue';
import SemanticZoomOverlayContent from './SemanticZoomOverlayContent.vue';
import SemanticZoomOverlayControls from './SemanticZoomOverlayControls.vue';
import SemanticZoomOverlayFailure from './SemanticZoomOverlayFailure.vue';
import SemanticZoomOverlayHeader from './SemanticZoomOverlayHeader.vue';
import { ScrollArea } from '@/components/ui/scroll-area';
const zoom = useSemanticZoomStore();
const pipeline = usePipelineStore();
const sessionStore = useSessionStore();
const appShellStore = useAppShellStore();
const promptDialogStore = usePromptDialogStore();
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
const reconnectSessionId = computed<string | null>(() => {
  if (nodeType.value === 'tool') return null;
  const sid = viewNode.value?.provenance?.detail.sessionId;
  if (!sid) return null;
  const trimmed = sid.trim();
  return trimmed.length > 0 ? trimmed : null;
});
const canConnectLive = computed(() =>
  viewNode.value?.status === 'active'
  && nodeType.value !== 'tool'
  && !!reconnectSessionId.value,
);
const canTakeRefinement = computed(() =>
  viewNode.value?.reconnected === true
  && viewNode.value.status === 'completed'
  && nodeType.value !== 'tool',
);
const reconnectBusy = ref(false);
async function handleReconnect(): Promise<void> {
  const sid = reconnectSessionId.value;
  if (!sid || reconnectBusy.value) return;
  reconnectBusy.value = true;
  try {
    const result = await sessionStore.focusOrResumeRemoteSession(
      sid,
      viewNode.value?.provenance?.agent ?? undefined,
      { suppressErrors: true },
    );
    if (result.status === 'activated' || result.status === 'resumed') {
      pipeline.markNodeReconnected(viewNode.value!.nodeId);
      appShellStore.openAgents();
      zoom.back();
    }
  } catch {
    // Unexpected failure — stay on the stored output in inspector mode.
  } finally {
    reconnectBusy.value = false;
  }
}
// Auto-connect to the live ACP session when the overlay opens for an active
// agent node.  Tool nodes are excluded — they have no interactive session.
watch(
  [() => zoom.isInspecting, canConnectLive],
  ([inspecting, canConnect]) => {
    if (inspecting && canConnect) {
      void handleReconnect();
    }
  },
  { flush: 'post' },
);
function handleTakeRefined(): void {
  const nodeId = viewNode.value?.nodeId;
  const latest = latestAssistantMessageForRemoteSession(sessionStore, viewNode.value?.provenance?.detail.sessionId);
  if (!nodeId || !latest) {
    toast.error('Unable to take refined output: reconnected session is no longer open.');
    return;
  }
  pipeline.replaceNodeOutput(nodeId, latest.content, {
    sessionId: latest.sessionId,
    takenAt: Date.now(),
  });
}
function handleDiscardRefined(): void {
  const nodeId = viewNode.value?.nodeId;
  if (nodeId) pipeline.clearNodeReconnected(nodeId);
}
const failureReason = computed<string | null>(() => {
  const reason = viewNode.value?.failureReason?.trim();
  return reason && reason.length > 0 ? reason : null;
});
const agentSummary = computed<string | null>(() => {
  const node = viewNode.value;
  if (!node) return null;
  const agent = node.provenance?.agent;
  if (!agent) return null;
  const model = 'model' in node && typeof node.model === 'string' && node.model.length > 0
    ? node.model
    : node.agentConfig?.model;
  return model ? `${agent} · ${model}` : agent;
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
  const requestedSessionIdInput = await promptDialogStore.requestValue({
    title: 'Fork pipeline session',
    description: 'Optional name for the forked pipeline session.',
    placeholder: 'my-forked-session',
    confirmLabel: 'Fork',
  });
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
watch(() => viewNode.value?.nodeId, () => {
  forkedSessionId.value = null;
  reconnectBusy.value = false;
  showGraftDialog.value = false;
  showPromotionDialog.value = false;
  promotionBusy.value = false;
});
</script>
<template>
  <Dialog :open="zoom.isInspecting && !!viewNode" @update:open="(open) => !open && zoom.back()">
    <DialogContent
      v-if="viewNode"
      class="flex max-h-[85vh] w-[80vw] max-w-[80vw] flex-col gap-0 overflow-hidden border border-border-subtle bg-card p-0 shadow-none"
      :show-close-button="false"
    >
      <DialogTitle class="sr-only">{{ displayLabel }}</DialogTitle>
      <DialogDescription class="sr-only">Pipeline node inspection details</DialogDescription>
      <SemanticZoomOverlayHeader
        :display-label="displayLabel"
        :node-type="nodeType"
        :status-label="statusLabel"
      />
      <div v-if="agentSummary" class="border-b border-border-subtle bg-accent px-4 py-2 font-display text-caption text-text-secondary">{{ agentSummary }}</div>
      <SemanticZoomOverlayFailure :failure-reason="failureReason" />
      <ScrollArea class="flex-1 min-h-0 overflow-y-auto">
        <div class="p-4">
          <SemanticZoomOverlayContent
            :assembled-prompt="assembledPrompt"
            :node-output="nodeOutput"
            :thinking-trace="thinkingTrace"
            :tool-provenance="toolProvenance"
            :workflow-steps="workflowSteps"
          />
        </div>
      </ScrollArea>
      <SemanticZoomOverlayControls
        :busy-action="busyAction"
        :can-connect-live="canConnectLive"
        :can-promote="canPromote"
        :can-take-refinement="canTakeRefinement"
        :forked-session-id="forkedSessionId"
        :graft-submitting="graftSubmitting"
        :has-results="hasResults"
        :promotion-busy="promotionBusy"
        :reconnect-busy="reconnectBusy"
        :reconnect-session-id="reconnectSessionId"
        :view-node-status="viewNode.status"
        @abandon="abandonNode"
        @fork="forkNode"
        @graft="openGraftDialog"
        @promote="openPromotionDialog"
        @reconnect="handleReconnect"
        @take-refinement="handleTakeRefined"
        @discard-refinement="handleDiscardRefined"
        @restart="restartNode"
        @rewind="rewindNode"
      />
      <GraftNodesDialog
        v-if="showGraftDialog && currentView"
        :busy="graftSubmitting"
        :existing-node-ids="existingNodeIds"
        :source-node-id="selectedNodeId"
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
    </DialogContent>
  </Dialog>
</template>
