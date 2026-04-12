<script setup lang="ts">
import { computed, ref } from 'vue';
import InspectorNodeActions from './InspectorNodeActions.vue';
import PromptAssemblyPreview from './PromptAssemblyPreview.vue';
import ThinkingTraceView from './ThinkingTraceView.vue';
import WorkflowStepProgress from './WorkflowStepProgress.vue';
import { usePipelineStore } from '../stores/pipeline';
import { useSessionStore } from '../stores/session';
import { useAppShellStore } from '../stores/app-shell';
import type {
  PipelineEdge,
  PipelineNodeType,
  PipelineNodeAgentConfig,
  PromptViewNode,
  SequentialThinkingViewNode,
  ToolViewNode,
  ViewNode,
  WorkflowViewNode,
} from '../lib/pipeline-types';
import { getViewNodeDisplayLabel } from '../lib/pipeline-types';

const props = defineProps<{
  selectedNodeId: string | null;
  selectedEdge: { source: string; target: string } | null;
}>();

const store = usePipelineStore();
const sessionStore = useSessionStore();
const appShellStore = useAppShellStore();
const openingAgents = ref(false);

const selectedViewNode = computed<ViewNode | null>(() =>
  props.selectedNodeId
    ? store.currentView?.nodes.find((node) => node.nodeId === props.selectedNodeId) ?? null
    : null,
);
const selectedWorkflowViewNode = computed<WorkflowViewNode | null>(() =>
  selectedViewNode.value && (selectedViewNode.value.type === 'workflow' || selectedViewNode.value.type == null)
    ? selectedViewNode.value
    : null,
);
const selectedPromptViewNode = computed<PromptViewNode | null>(() =>
  selectedViewNode.value?.type === 'prompt' ? selectedViewNode.value : null,
);
const selectedThinkingViewNode = computed<SequentialThinkingViewNode | null>(() =>
  selectedViewNode.value?.type === 'sequentialThinking' ? selectedViewNode.value : null,
);
const selectedToolViewNode = computed<ToolViewNode | null>(() =>
  selectedViewNode.value?.type === 'tool' ? selectedViewNode.value : null,
);
const selectedViewEdge = computed<PipelineEdge | null>(() =>
  props.selectedEdge
    ? store.currentView?.edges.find(
        (edge) => edge.source === props.selectedEdge?.source && edge.target === props.selectedEdge?.target,
      ) ?? null
    : null,
);
const selectedEdgeSourceNode = computed<ViewNode | null>(() =>
  selectedViewEdge.value
    ? store.currentView?.nodes.find((node) => node.nodeId === selectedViewEdge.value?.source) ?? null
    : null,
);
const selectedEdgeTargetNode = computed<ViewNode | null>(() =>
  selectedViewEdge.value
    ? store.currentView?.nodes.find((node) => node.nodeId === selectedViewEdge.value?.target) ?? null
    : null,
);
const selectedViewNodeLabel = computed(() =>
  selectedViewNode.value
    ? getViewNodeDisplayLabel(selectedViewNode.value)
    : '',
);
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
const selectedToolResolvedArgs = computed<Record<string, unknown> | null>(() => {
  if (!selectedToolViewNode.value) return null;
  return selectedToolViewNode.value.toolProvenance?.resolvedArgs
    ?? selectedToolViewNode.value.toolArgs
    ?? null;
});
const showToolDirectExecutionExplanation = computed(() => (
  !!selectedToolViewNode.value && !selectedNodeReconnectSessionId.value
));
const showExpiredSessionExplanation = computed(() => (
  !!selectedViewNode.value
  && !selectedNodeCanReconnect.value
  && !!selectedNodeReconnectSessionId.value
));
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

function formatNodeType(type?: PipelineNodeType): string {
  return type === 'sequentialThinking'
    ? 'Sequential Thinking'
    : type === 'prompt'
      ? 'Prompt'
      : type === 'tool'
        ? 'Tool'
        : 'Workflow';
}

function displayValue(value?: string | number | null): string {
  return value === undefined || value === null || value === '' ? '—' : String(value);
}

function formatJsonValue(value?: Record<string, unknown>): string {
  return value ? JSON.stringify(value, null, 2) : '—';
}

function formatUnknownJson(value: unknown): string {
  return value === undefined ? '—' : JSON.stringify(value, null, 2);
}

function hasAssembledPrompt(value?: string | null): value is string {
  return value !== undefined && value !== null;
}

function hasResponseSummary(value?: string | null): value is string {
  return value !== undefined && value !== null && value.trim().length > 0;
}

function formatStatusLabel(status: string): string {
  return status.charAt(0).toUpperCase() + status.slice(1);
}

function truncateText(value: string, limit = 120): string {
  return value.length <= limit ? value : `${value.slice(0, limit - 1).trimEnd()}…`;
}

function deriveSummaryText(node: ViewNode): string {
  const firstLine = getOperatorOutputText(node)
    .split(/\r?\n/, 1)[0]
    ?.trim();

  return firstLine ? truncateText(firstLine) : 'Waiting for output…';
}

function getToolResponseSummary(node: ViewNode | null): string | null {
  if (node?.type !== 'tool') return null;

  const responseSummary = node.toolProvenance?.responseSummary;
  return hasResponseSummary(responseSummary) ? responseSummary : null;
}

function getOperatorOutputText(node: ViewNode | null): string {
  if (!node) return '';

  if (typeof node.output === 'string' && node.output.trim().length > 0) {
    return node.output;
  }

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

  const segments = path
    .split('.')
    .flatMap((segment) => segment.split(/\[|\]/).filter(Boolean));
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

function getAgentConfig(node: ViewNode): PipelineNodeAgentConfig | undefined {
  return 'agentConfig' in node ? node.agentConfig : undefined;
}

function getAgentName(node: ViewNode): string | undefined {
  return ('agent' in node && typeof node.agent === 'string' && node.agent.length > 0)
    ? node.agent
    : node.provenance?.agent ?? undefined;
}

function getModelName(node: ViewNode): string | undefined {
  return ('model' in node && typeof node.model === 'string' && node.model.length > 0)
    ? node.model
    : getAgentConfig(node)?.model;
}

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
  return normalizeSessionId(node.provenance?.detail.sessionId)
    ?? (node.type !== 'tool' ? normalizeSessionId(node.acpSessionId) : null)
    ?? null;
}

async function openInAgentsTab(): Promise<void> {
  const reconnectSessionId = selectedNodeReconnectSessionId.value;
  if (!reconnectSessionId || !selectedNodeCanReconnect.value || openingAgents.value) {
    return;
  }

  openingAgents.value = true;
  try {
    const result = await sessionStore.focusOrResumeRemoteSession(reconnectSessionId);
    if (result.status === 'activated' || result.status === 'resumed') {
      appShellStore.openAgents();
    }
  } finally {
    openingAgents.value = false;
  }
}

function copyToClipboard(value: string): void {
  navigator.clipboard.writeText(value);
}
</script>

<template>
  <section v-if="inspectorMode === 'server-overview' && store.currentView" class="panel">
    <h2>Pipeline Overview</h2>
    <div class="summary-row"><span>Pipeline ID</span><strong class="mono">{{ store.currentView.pipelineId }}</strong></div>
    <div class="summary-row">
      <span>Status</span>
      <strong :class="['status-text', `status-text--${store.currentView.status}`]">{{ store.currentView.status }}</strong>
    </div>
    <div class="summary-row"><span>Progress</span><strong>{{ store.currentView.completed.length }} / {{ store.currentView.nodes.length }}</strong></div>
    <div class="summary-row"><span>Running</span><strong>{{ store.currentView.running ? 'Yes' : 'No' }}</strong></div>
  </section>

  <section v-else-if="inspectorMode === 'server-node' && selectedViewNode" class="panel">
    <h2>{{ selectedViewNodeLabel || selectedViewNode.nodeId || 'Node Details' }}</h2>
    <section class="status-headline">
      <div class="status-headline__meta">
        <strong :class="['status-text', `status-text--${selectedViewNode.status}`]">{{ formatStatusLabel(selectedViewNode.status) }}</strong>
        <strong class="type-pill">{{ formatNodeType(selectedViewNode.type ?? 'workflow') }}</strong>
      </div>
      <p>{{ selectedNodeHeadline }}</p>
    </section>

    <div v-if="selectedNodeAgentSummary" class="summary-row">
      <span>Agent</span>
      <strong>{{ selectedNodeAgentSummary }}</strong>
    </div>

    <template v-if="selectedToolViewNode">
      <div v-if="showToolDirectExecutionExplanation" class="operator-callout">
        <strong>Tool node — executed directly, no interactive session</strong>
        <span>
          {{ selectedToolViewNode.toolProvenance?.toolName ?? selectedToolViewNode.toolName }}
        </span>
      </div>
      <div class="summary-row">
        <span>Tool</span>
        <strong>{{ selectedToolViewNode.toolProvenance?.toolName ?? selectedToolViewNode.toolName }}</strong>
      </div>
      <div class="read-block">
        <span>Resolved args</span>
        <pre>{{ formatJsonValue(selectedToolResolvedArgs ?? undefined) }}</pre>
      </div>
    </template>

    <details class="output-block" :open="selectedNodeOutput.length > 0">
      <summary>Output</summary>
      <pre v-if="selectedNodeOutput.length > 0">{{ selectedNodeOutput }}</pre>
      <pre v-else>Output will appear when this node completes</pre>
    </details>

    <div v-if="showExpiredSessionExplanation" class="operator-callout">
      <strong>Session ended — results preserved above</strong>
      <code class="mono">{{ selectedNodeReconnectSessionId }}</code>
    </div>

    <div v-if="selectedNodeCanReconnect && selectedNodeReconnectSessionId" class="action-row">
      <button class="btn btn-primary" type="button" :disabled="openingAgents" @click="openInAgentsTab">
        {{ openingAgents ? 'Opening…' : 'Open in Agents tab' }}
      </button>
    </div>

    <InspectorNodeActions :node="selectedViewNode" :node-id="selectedViewNode.nodeId" />

    <details v-if="shouldShowExpertDetails" class="expert-disclosure">
      <summary>Technical Details</summary>

      <div class="expert-content">
        <div class="summary-row">
          <span>Session ID</span>
          <div class="copy-row">
            <strong class="mono">{{ selectedViewNode.sessionId }}</strong>
            <button class="btn btn-ghost copy-button" type="button" @click="copyToClipboard(selectedViewNode.sessionId)">
              Copy
            </button>
          </div>
        </div>

        <div v-if="selectedViewNode.acpSessionId" class="summary-row">
          <span>ACP session</span>
          <div class="copy-row">
            <strong class="mono">{{ selectedViewNode.acpSessionId }}</strong>
            <button class="btn btn-ghost copy-button" type="button" @click="copyToClipboard(selectedViewNode.acpSessionId ?? '')">
              Copy
            </button>
          </div>
        </div>

        <div v-if="selectedViewNode.thinkingSessionId" class="summary-row">
          <span>Thinking session</span>
          <div class="copy-row">
            <strong class="mono">{{ selectedViewNode.thinkingSessionId }}</strong>
            <button class="btn btn-ghost copy-button" type="button" @click="copyToClipboard(selectedViewNode.thinkingSessionId ?? '')">
              Copy
            </button>
          </div>
        </div>

        <div v-if="selectedViewNode.attempts > 1" class="summary-row">
          <span>Attempts</span>
          <strong>{{ selectedViewNode.attempts }}</strong>
        </div>

        <template v-if="selectedWorkflowViewNode">
          <div class="summary-row"><span>Workflow ID</span><strong>{{ selectedWorkflowViewNode.workflowId }}</strong></div>
          <div class="summary-row"><span>ACP client</span><strong>{{ displayValue(selectedWorkflowViewNode.acpClient) }}</strong></div>
          <div v-if="selectedWorkflowViewNode.prompt" class="read-block"><span>Prompt template</span><pre>{{ displayValue(selectedWorkflowViewNode.prompt) }}</pre></div>
          <PromptAssemblyPreview
            v-if="hasAssembledPrompt(selectedWorkflowViewNode.assembled_prompt)"
            :assembled-prompt="selectedWorkflowViewNode.assembled_prompt"
          />
          <WorkflowStepProgress
            v-if="selectedWorkflowViewNode.workflow_steps != null"
            :steps="selectedWorkflowViewNode.workflow_steps"
          />
        </template>

        <template v-else-if="selectedPromptViewNode">
          <div class="read-block"><span>Prompt template</span><pre>{{ selectedPromptViewNode.prompt }}</pre></div>
          <PromptAssemblyPreview
            v-if="hasAssembledPrompt(selectedPromptViewNode.assembled_prompt)"
            :assembled-prompt="selectedPromptViewNode.assembled_prompt"
          />
        </template>

        <template v-else-if="selectedThinkingViewNode">
          <div class="read-block"><span>Topic</span><pre>{{ selectedThinkingViewNode.topic }}</pre></div>
          <div class="summary-row"><span>Total thoughts</span><strong>{{ displayValue(selectedThinkingViewNode.totalThoughts) }}</strong></div>
          <div class="summary-row"><span>Analysis type</span><strong>{{ displayValue(selectedThinkingViewNode.analysisType) }}</strong></div>
          <PromptAssemblyPreview
            v-if="hasAssembledPrompt(selectedThinkingViewNode.assembled_prompt)"
            :assembled-prompt="selectedThinkingViewNode.assembled_prompt"
          />
          <ThinkingTraceView
            v-if="selectedThinkingViewNode.thinking_trace != null"
            :trace="selectedThinkingViewNode.thinking_trace"
          />
        </template>

        <template v-else-if="selectedToolViewNode">
          <div class="summary-row"><span>Tool name</span><strong>{{ selectedToolViewNode.toolName }}</strong></div>
          <div class="read-block"><span>Tool args</span><pre>{{ formatJsonValue(selectedToolViewNode.toolArgs) }}</pre></div>
          <div v-if="hasResponseSummary(selectedToolViewNode.toolProvenance?.responseSummary)" class="read-block">
            <span>Response summary</span>
            <pre>{{ selectedToolViewNode.toolProvenance?.responseSummary }}</pre>
          </div>
        </template>
      </div>
    </details>
  </section>

  <section v-else-if="inspectorMode === 'server-edge' && selectedViewEdge" class="panel">
    <h2>Edge Details</h2>
    <div class="summary-row"><span>Path</span><strong>{{ selectedEdgePathLabel }}</strong></div>
    <div class="summary-row"><span>Source status</span><strong :class="['status-text', `status-text--${selectedEdgeSourceNode?.status ?? 'pending'}`]">{{ selectedEdgeSourceNode?.status ?? '—' }}</strong></div>
    <div class="read-block">
      <span>Mapping</span>
      <div v-if="selectedEdgeMappings.length > 0" class="mapping-list">
        <div v-for="[targetField, sourcePath] in selectedEdgeMappings" :key="`${targetField}:${sourcePath}`" class="mapping-row">
          <strong>{{ sourcePath }}</strong>
          <span>→</span>
          <code>{{ targetField }}</code>
        </div>
      </div>
      <pre v-else>No mapping configured</pre>
    </div>
    <div class="read-block">
      <span>Source output</span>
      <pre v-if="selectedEdgeSourceNode?.status === 'completed' && selectedEdgeSourceNode.output">{{ selectedEdgeSourceNode.output }}</pre>
      <pre v-else>Source output will appear after the source node completes</pre>
    </div>
    <div class="read-block">
      <span>Resolved target input</span>
      <pre v-if="selectedEdgeTargetNode?.type === 'tool'">{{ formatUnknownJson(selectedEdgeResolvedInput) }}</pre>
      <pre v-else>Runtime input mapping applies to tool targets only</pre>
    </div>
  </section>
 
  <section v-else class="panel empty-state"><h2>Inspector</h2><p>Select a node or edge to inspect</p></section>
</template>

<style scoped>
.panel { padding: 1rem; display: flex; flex-direction: column; gap: 0.875rem; }
.panel h2 { font-size: var(--text-xl); font-weight: 600; }
.field,.read-block,.output-block,.expert-disclosure { display: flex; flex-direction: column; gap: 0.375rem; color: var(--text-secondary); font-size: var(--text-sm); }
.read-block pre,.output-block { border: 1px solid var(--border-default); border-radius: 8px; padding: 0.75rem; background: var(--bg-main); }
.read-block span { padding: 0 0.25rem; color: var(--text-primary); }
.summary-row { display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; }
.summary-row span,.output-block summary,.expert-disclosure summary { color: var(--text-secondary); }
.type-pill { padding: 0.2rem 0.6rem; font-size: var(--text-sm); border: 1px solid var(--border-default); border-radius: 8px; background: var(--bg-main); }
.read-block pre,.output-block pre { margin: 0; white-space: pre-wrap; word-break: break-word; overflow-wrap: break-word; font: inherit; color: var(--text-primary); }
.mapping-list { display: flex; flex-direction: column; gap: 0.5rem; }
.mapping-row { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; }
.summary-row strong,.copy-row strong,.operator-callout span,.operator-callout code { min-width: 0; word-break: break-word; overflow-wrap: break-word; }
.mono { font-family: var(--font-mono); word-break: break-word; overflow-wrap: break-word; }
.status-headline { display: flex; flex-direction: column; gap: 0.5rem; padding: 0.875rem; border: 1px solid var(--border-default); border-radius: 10px; background: var(--bg-main); }
.status-headline__meta { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; }
.status-headline p { margin: 0; color: var(--text-primary); }
.operator-callout { display: flex; flex-direction: column; gap: 0.35rem; padding: 0.75rem; border: 1px solid var(--border-default); border-radius: 8px; background: var(--bg-main); }
.operator-callout strong { color: var(--text-primary); }
.action-row { display: flex; flex-wrap: wrap; gap: 0.75rem; }
.expert-disclosure { padding: 0.875rem; border: 1px solid var(--border-default); border-radius: 10px; background: var(--bg-main); }
.expert-content { display: flex; flex-direction: column; gap: 0.875rem; margin-top: 0.75rem; }
.copy-row { display: flex; align-items: center; gap: 0.5rem; justify-content: flex-end; flex-wrap: wrap; }
.copy-button { padding: 0.25rem 0.5rem; }
.empty-state { justify-content: center; min-height: 220px; }
.empty-state p { color: var(--text-secondary); }
</style>
