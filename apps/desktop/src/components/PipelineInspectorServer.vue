<script setup lang="ts">
import { computed } from 'vue';
import InspectorNodeActions from './InspectorNodeActions.vue';
import PromptAssemblyPreview from './PromptAssemblyPreview.vue';
import ThinkingTraceView from './ThinkingTraceView.vue';
import WorkflowStepProgress from './WorkflowStepProgress.vue';
import { usePipelineStore } from '../stores/pipeline';
import type {
  PipelineEdge,
  PipelineNodeType,
  PromptViewNode,
  SequentialThinkingViewNode,
  ToolViewNode,
  ViewNode,
  WorkflowViewNode,
} from '../lib/pipeline-types';

const props = defineProps<{
  selectedNodeId: string | null;
  selectedEdge: { source: string; target: string } | null;
}>();

const store = usePipelineStore();

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
  return value !== undefined && value !== null && value.length > 0;
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
    <h2>Node Details</h2>
    <div class="summary-row"><span>Node ID</span><strong class="mono">{{ selectedViewNode.nodeId }}</strong></div>
    <div class="summary-row"><span>Type</span><strong class="type-pill">{{ formatNodeType(selectedViewNode.type ?? 'workflow') }}</strong></div>
    <div class="summary-row">
      <span>Status</span>
      <strong :class="['status-text', `status-text--${selectedViewNode.status}`]">{{ selectedViewNode.status }}</strong>
    </div>
    <div class="summary-row"><span>Session</span><strong class="mono">{{ selectedViewNode.sessionId }}</strong></div>
    <div v-if="selectedViewNode.acpSessionId" class="summary-row">
      <span>ACP session</span>
      <strong class="mono">{{ selectedViewNode.acpSessionId }}</strong>
    </div>
    <div v-if="selectedViewNode.thinkingSessionId" class="summary-row">
      <span>Thinking session</span>
      <strong class="mono">{{ selectedViewNode.thinkingSessionId }}</strong>
    </div>
    <div class="summary-row"><span>Attempts</span><strong>{{ selectedViewNode.attempts }}</strong></div>

    <template v-if="selectedWorkflowViewNode">
      <div class="summary-row"><span>Workflow ID</span><strong>{{ selectedWorkflowViewNode.workflowId }}</strong></div>
      <div class="summary-row"><span>ACP client</span><strong>{{ displayValue(selectedWorkflowViewNode.acpClient) }}</strong></div>
      <div class="read-block"><span>Prompt</span><pre>{{ displayValue(selectedWorkflowViewNode.prompt) }}</pre></div>
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
      <div class="summary-row"><span>Agent</span><strong>{{ selectedPromptViewNode.agent }}</strong></div>
      <div class="read-block"><span>Prompt template</span><pre>{{ selectedPromptViewNode.prompt }}</pre></div>
      <PromptAssemblyPreview
        v-if="hasAssembledPrompt(selectedPromptViewNode.assembled_prompt)"
        :assembled-prompt="selectedPromptViewNode.assembled_prompt"
      />
    </template>

    <template v-else-if="selectedThinkingViewNode">
      <div class="read-block"><span>Topic</span><pre>{{ selectedThinkingViewNode.topic }}</pre></div>
      <div class="summary-row"><span>Agent</span><strong>{{ selectedThinkingViewNode.agent }}</strong></div>
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
      <template v-if="selectedToolViewNode.toolProvenance">
        <div class="summary-row">
          <span>Executed tool</span>
          <strong>{{ selectedToolViewNode.toolProvenance.toolName }}</strong>
        </div>
        <div class="read-block">
          <span>Resolved args</span>
          <pre>{{ formatJsonValue(selectedToolViewNode.toolProvenance.resolvedArgs ?? undefined) }}</pre>
        </div>
        <div v-if="hasResponseSummary(selectedToolViewNode.toolProvenance.responseSummary)" class="read-block">
          <span>Response summary</span>
          <pre>{{ selectedToolViewNode.toolProvenance.responseSummary }}</pre>
        </div>
      </template>
    </template>

    <details v-if="typeof selectedViewNode.output === 'string' && selectedViewNode.output.trim().length > 0" class="output-block"><summary>Output</summary><pre>{{ selectedViewNode.output }}</pre></details>
    <InspectorNodeActions :node="selectedViewNode" :node-id="selectedViewNode.nodeId" />
  </section>

  <section v-else-if="inspectorMode === 'server-edge' && selectedViewEdge" class="panel">
    <h2>Edge Details</h2>
    <div class="summary-row"><span>Path</span><strong>{{ selectedViewEdge.source }} → {{ selectedViewEdge.target }}</strong></div>
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
.field,.read-block,.output-block { display: flex; flex-direction: column; gap: 0.375rem; color: var(--text-secondary); font-size: var(--text-sm); }
.read-block pre,.output-block { border: 1px solid var(--border-color); border-radius: 8px; padding: 0.75rem; background: var(--bg-main); }
.read-block span { padding: 0 0.25rem; color: var(--text-primary); }
.summary-row { display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; }
.summary-row span,.output-block summary { color: var(--text-secondary); }
.type-pill { padding: 0.2rem 0.6rem; font-size: var(--text-sm); border: 1px solid var(--border-color); border-radius: 8px; background: var(--bg-main); }
.read-block pre,.output-block pre { margin: 0; white-space: pre-wrap; word-break: break-word; font: inherit; color: var(--text-primary); }
.mapping-list { display: flex; flex-direction: column; gap: 0.5rem; }
.mapping-row { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; }
.mono { font-family: var(--font-mono); word-break: break-all; }
.empty-state { justify-content: center; min-height: 220px; }
.empty-state p { color: var(--text-secondary); }
</style>
