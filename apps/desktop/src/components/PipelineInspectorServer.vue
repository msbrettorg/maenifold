<script setup lang="ts">
import { computed, ref } from 'vue';
import PipelineInspectorServerEdge from './PipelineInspectorServerEdge.vue';
import PipelineInspectorServerNode from './PipelineInspectorServerNode.vue';
import PipelineInspectorServerOverview from './PipelineInspectorServerOverview.vue';
import { usePipelineStore } from '../stores/pipeline';
import { useSessionStore } from '../stores/session';
import { useAppShellStore } from '../stores/app-shell';
import type { PipelineEdge, PipelineNodeAgentConfig, PromptViewNode, SequentialThinkingViewNode, ToolViewNode, ViewNode, WorkflowViewNode } from '../lib/pipeline-types';
import { getViewNodeDisplayLabel } from '../lib/pipeline-types';

const props = defineProps<{
  selectedNodeId: string | null;
  selectedEdge: { source: string; target: string } | null;
}>();

const store = usePipelineStore();
const sessionStore = useSessionStore();
const appShellStore = useAppShellStore();
const openingAgents = ref(false);

const selectedViewNode = computed<ViewNode | null>(() => props.selectedNodeId ? store.currentView?.nodes.find((node) => node.nodeId === props.selectedNodeId) ?? null : null);
const selectedWorkflowViewNode = computed<WorkflowViewNode | null>(() => selectedViewNode.value && (selectedViewNode.value.type === 'workflow' || selectedViewNode.value.type == null) ? selectedViewNode.value : null);
const selectedPromptViewNode = computed<PromptViewNode | null>(() => selectedViewNode.value?.type === 'prompt' ? selectedViewNode.value : null);
const selectedThinkingViewNode = computed<SequentialThinkingViewNode | null>(() => selectedViewNode.value?.type === 'sequentialThinking' ? selectedViewNode.value : null);
const selectedToolViewNode = computed<ToolViewNode | null>(() => selectedViewNode.value?.type === 'tool' ? selectedViewNode.value : null);
const selectedViewEdge = computed<PipelineEdge | null>(() => props.selectedEdge ? store.currentView?.edges.find((edge) => edge.source === props.selectedEdge?.source && edge.target === props.selectedEdge?.target) ?? null : null);
const selectedEdgeSourceNode = computed<ViewNode | null>(() => selectedViewEdge.value ? store.currentView?.nodes.find((node) => node.nodeId === selectedViewEdge.value?.source) ?? null : null);
const selectedEdgeTargetNode = computed<ViewNode | null>(() => selectedViewEdge.value ? store.currentView?.nodes.find((node) => node.nodeId === selectedViewEdge.value?.target) ?? null : null);
const selectedViewNodeLabel = computed(() => selectedViewNode.value ? getViewNodeDisplayLabel(selectedViewNode.value) : '');
const selectedEdgePathLabel = computed(() => {
  if (!selectedViewEdge.value) return '';
  const source = selectedEdgeSourceNode.value
    ? getViewNodeDisplayLabel(selectedEdgeSourceNode.value)
    : selectedViewEdge.value.source;
  const target = selectedEdgeTargetNode.value
    ? getViewNodeDisplayLabel(selectedEdgeTargetNode.value)
    : selectedViewEdge.value.target;
  return `${source} → ${target}`;
});
const selectedEdgeMappings = computed(() => Object.entries(selectedViewEdge.value?.mapping ?? {}));
const selectedEdgeResolvedInput = computed<Record<string, unknown> | null>(() => {
  if (selectedEdgeTargetNode.value?.type !== 'tool') return null;

  const targetInput = { ...(selectedEdgeTargetNode.value.toolArgs ?? {}) };
  const sourceOutput = parseJsonValue(selectedEdgeSourceNode.value?.output);
  for (const [targetField, sourcePath] of selectedEdgeMappings.value) {
    targetInput[targetField] = resolveDotPath(sourceOutput, sourcePath);
  }
  return targetInput;
});
const inspectorMode = computed(() => {
  if (selectedViewNode.value) return 'server-node';
  if (selectedViewEdge.value) return 'server-edge';
  if (!props.selectedNodeId && !props.selectedEdge && store.currentView) return 'server-overview';
  return 'fallback';
});
const selectedNodeOutput = computed(() => getOperatorOutputText(selectedViewNode.value));
const selectedNodeHeadline = computed(() => {
  if (!selectedViewNode.value) return '';
  return `${formatStatusLabel(selectedViewNode.value.status)} — ${deriveSummaryText(selectedViewNode.value)}`;
});
const selectedNodeReconnectSessionId = computed(() => getReconnectSessionId(selectedViewNode.value));
const selectedNodeCanReconnect = computed(() => canReconnectToAgentSession(selectedViewNode.value));
const selectedNodeAgentSummary = computed(() => {
  if (!selectedViewNode.value) return null;

  const agent = getAgentName(selectedViewNode.value);
  if (!agent) return null;

  const model = getModelName(selectedViewNode.value);
  return model ? `${agent} · ${model}` : agent;
});
const selectedToolResolvedArgs = computed<Record<string, unknown> | null>(() => selectedToolViewNode.value?.toolProvenance?.resolvedArgs ?? selectedToolViewNode.value?.toolArgs ?? null);
const showToolDirectExecutionExplanation = computed(() => !!selectedToolViewNode.value && !selectedNodeReconnectSessionId.value);
const showExpiredSessionExplanation = computed(() => !!selectedViewNode.value && !selectedNodeCanReconnect.value && !!selectedNodeReconnectSessionId.value);
const shouldShowExpertDetails = computed(() => {
  const node = selectedViewNode.value;
  if (!node) return false;

  return (
    node.sessionId.length > 0
    || !!node.acpSessionId
    || !!node.thinkingSessionId
    || node.attempts > 1
    || !!selectedWorkflowViewNode.value?.workflow_steps?.length
    || !!selectedWorkflowViewNode.value?.prompt
    || !!selectedWorkflowViewNode.value?.assembled_prompt
    || !!selectedPromptViewNode.value?.prompt
    || !!selectedPromptViewNode.value?.assembled_prompt
    || !!selectedThinkingViewNode.value?.topic
    || !!selectedThinkingViewNode.value?.assembled_prompt
    || !!selectedThinkingViewNode.value?.thinking_trace?.length
    || !!selectedToolViewNode.value?.toolArgs
    || !!selectedToolViewNode.value?.toolProvenance?.responseSummary
  );
});

function formatUnknownJson(value: unknown): string { return value === undefined ? '—' : JSON.stringify(value, null, 2); }
function formatStatusLabel(status: string): string { return status.charAt(0).toUpperCase() + status.slice(1); }
function truncateText(value: string, limit = 120): string { return value.length <= limit ? value : `${value.slice(0, limit - 1).trimEnd()}…`; }
function deriveSummaryText(node: ViewNode): string {
  const firstLine = getOperatorOutputText(node).split(/\r?\n/, 1)[0]?.trim();
  return firstLine ? truncateText(firstLine) : 'Waiting for output…';
}

function getToolResponseSummary(node: ViewNode | null): string | null {
  if (node?.type !== 'tool') return null;
  const responseSummary = node.toolProvenance?.responseSummary;
  return responseSummary && responseSummary.trim().length > 0 ? responseSummary : null;
}

function getOperatorOutputText(node: ViewNode | null): string {
  if (!node) return '';
  if (typeof node.output === 'string' && node.output.trim().length > 0) return node.output;
  return getToolResponseSummary(node) ?? '';
}

function parseJsonValue(value?: string | null): unknown {
  if (!value) return undefined;
  try {
    return JSON.parse(value);
  } catch {
    return value;
  }
}

function resolveDotPath(input: unknown, path: string): unknown {
  if (!path.trim()) return null;
  const segments = path.split('.').flatMap((segment) => segment.split(/\[|\]/).filter(Boolean));
  let current = input;
  for (const segment of segments) {
    if (Array.isArray(current)) {
      current = current[Number(segment)];
      continue;
    }
    if (current && typeof current === 'object') {
      current = (current as Record<string, unknown>)[segment];
      continue;
    }
    return null;
  }

  return current ?? null;
}

function getAgentConfig(node: ViewNode): PipelineNodeAgentConfig | undefined { return 'agentConfig' in node ? node.agentConfig : undefined; }
function getAgentName(node: ViewNode): string | undefined { return ('agent' in node && typeof node.agent === 'string' && node.agent.length > 0) ? node.agent : node.provenance?.agent ?? undefined; }
function getModelName(node: ViewNode): string | undefined { return ('model' in node && typeof node.model === 'string' && node.model.length > 0) ? node.model : getAgentConfig(node)?.model; }

function normalizeSessionId(value?: string | null): string | null {
  if (!value) return null;
  const trimmedValue = value.trim();
  return trimmedValue.length > 0 ? trimmedValue : null;
}

function canReconnectToAgentSession(node: ViewNode | null): boolean {
  if (!node) return false;
  return node.provenance?.detail.canReconnect ?? (node.type !== 'tool' && !!normalizeSessionId(node.acpSessionId));
}

function getReconnectSessionId(node: ViewNode | null): string | null {
  if (!node) return null;
  return normalizeSessionId(node.provenance?.detail.sessionId) ?? (node.type !== 'tool' ? normalizeSessionId(node.acpSessionId) : null) ?? null;
}

async function openInAgentsTab(): Promise<void> {
  const reconnectSessionId = selectedNodeReconnectSessionId.value;
  if (!reconnectSessionId || !selectedNodeCanReconnect.value || openingAgents.value) return;
  openingAgents.value = true;
  try {
    const result = await sessionStore.focusOrResumeRemoteSession(reconnectSessionId);
    if (result.status === 'activated' || result.status === 'resumed') appShellStore.openAgents();
  } finally {
    openingAgents.value = false;
  }
}

function copyToClipboard(value: string): void {
  navigator.clipboard.writeText(value);
}
</script>

<template>
  <section class="pipeline-inspector-server" role="region" aria-label="Server inspector">
    <PipelineInspectorServerOverview v-if="inspectorMode === 'server-overview' && store.currentView" :pipeline-id="store.currentView.pipelineId" :status="store.currentView.status" :completed-count="store.currentView.completed.length" :node-count="store.currentView.nodes.length" :running="store.currentView.running" />

    <PipelineInspectorServerNode
      v-else-if="inspectorMode === 'server-node' && selectedViewNode"
      :selected-view-node="selectedViewNode"
      :selected-view-node-label="selectedViewNodeLabel"
      :selected-node-headline="selectedNodeHeadline"
      :selected-node-output="selectedNodeOutput"
      :selected-node-agent-summary="selectedNodeAgentSummary"
      :selected-node-reconnect-session-id="selectedNodeReconnectSessionId"
      :selected-node-can-reconnect="selectedNodeCanReconnect"
      :opening-agents="openingAgents"
      :show-tool-direct-execution-explanation="showToolDirectExecutionExplanation"
      :show-expired-session-explanation="showExpiredSessionExplanation"
      :should-show-expert-details="shouldShowExpertDetails"
      :selected-tool-resolved-args="selectedToolResolvedArgs"
      :selected-workflow-view-node="selectedWorkflowViewNode"
      :selected-prompt-view-node="selectedPromptViewNode"
      :selected-thinking-view-node="selectedThinkingViewNode"
      :selected-tool-view-node="selectedToolViewNode"
      @open-agents="openInAgentsTab"
      @copy="copyToClipboard"
    />

    <PipelineInspectorServerEdge v-else-if="inspectorMode === 'server-edge' && selectedViewEdge" :edge-path-label="selectedEdgePathLabel" :source-status="selectedEdgeSourceNode?.status ?? 'pending'" :edge-mappings="selectedEdgeMappings" :source-output="selectedEdgeSourceNode?.status === 'completed' && selectedEdgeSourceNode.output ? selectedEdgeSourceNode.output : null" :tool-target="selectedEdgeTargetNode?.type === 'tool'" :resolved-target-input="formatUnknownJson(selectedEdgeResolvedInput)" />

    <section v-else class="panel empty-state" role="region" aria-label="Server inspector empty state"><h2>Inspector</h2><p>Select a node or edge to inspect</p></section>
  </section>
</template>

<style scoped>
.pipeline-inspector-server {
  width: 100%;
  height: 100%;
  min-height: 0;
  padding: 0.75rem;
}
.panel,.empty-state { padding: 1rem; display: flex; flex-direction: column; gap: 0.875rem; border-radius: 4px; background: var(--pipeline-inspector-surface); }
.empty-state {
  justify-content: center;
  min-height: 220px;
  border: 1px solid var(--pipeline-inspector-ghost-border);
  background: var(--pipeline-inspector-well);
}
.empty-state p { color: var(--muted-foreground); }
</style>
