<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useAgentCapabilitiesStore } from '../stores/agent-capabilities';
import { usePipelineStore } from '../stores/pipeline';
import { useConfigStore } from '../stores/config';
import { groupToolsByCategory } from '../lib/tool-categories';
import type { PipelineEdge, PipelineNode, PipelineNodeType } from '../lib/pipeline-types';
import { getNodeType, isPromptNode, isSequentialThinkingNode, isToolNode, isWorkflowNode } from '../lib/pipeline-types';
import EdgeMappingEditor from './EdgeMappingEditor.vue';
import PipelineInspectorServer from './PipelineInspectorServer.vue';
import SchemaForm from './SchemaForm.vue';

const EMPTY_TOOL_ARGS: Readonly<Record<string, unknown>> = Object.freeze({});

const props = defineProps<{
  selectedNodeId: string | null;
  selectedEdge: { source: string; target: string } | null;
}>();

const store = usePipelineStore();
const configStore = useConfigStore();
const capabilitiesStore = useAgentCapabilitiesStore();
const workflowListId = 'pipeline-inspector-workflows';
const toolOptions = computed(() => groupToolsByCategory(store.tools));
const attemptedToolLoad = ref(false);
const loadingToolSchemas = ref(false);

const selectedDesignNode = computed(() =>
  props.selectedNodeId
    ? store.localDefinition.nodes.find((node) => node.id === props.selectedNodeId) ?? null
    : null,
);
const selectedDesignEdge = computed(() =>
  props.selectedEdge
    ? store.localDefinition.edges.find(
        (edge) => edge.source === props.selectedEdge?.source && edge.target === props.selectedEdge?.target,
      ) ?? null
    : null,
);
const selectedWorkflowDesignNode = computed(() =>
  selectedDesignNode.value && isWorkflowNode(selectedDesignNode.value) ? selectedDesignNode.value : null,
);
const selectedPromptDesignNode = computed(() =>
  selectedDesignNode.value && isPromptNode(selectedDesignNode.value) ? selectedDesignNode.value : null,
);
const selectedThinkingDesignNode = computed(() =>
  selectedDesignNode.value && isSequentialThinkingNode(selectedDesignNode.value)
    ? selectedDesignNode.value
    : null,
);
const selectedToolDesignNode = computed(() =>
  selectedDesignNode.value && isToolNode(selectedDesignNode.value) ? selectedDesignNode.value : null,
);
const selectedToolSchema = computed(
  () => store.tools.find((tool) => tool.name === selectedToolDesignNode.value?.toolName)?.inputSchema ?? null,
);
const selectedToolArgsData = computed<Record<string, unknown>>(
  () => selectedToolDesignNode.value?.toolArgs ?? EMPTY_TOOL_ARGS,
);
const selectedEdgeSourceNode = computed(() =>
  selectedDesignEdge.value
    ? store.localDefinition.nodes.find((node) => node.id === selectedDesignEdge.value?.source) ?? null
    : null,
);
const selectedEdgeTargetNode = computed(() =>
  selectedDesignEdge.value
    ? store.localDefinition.nodes.find((node) => node.id === selectedDesignEdge.value?.target) ?? null
    : null,
);
const inspectorMode = computed(() => {
  if (store.designMode) {
    if (selectedDesignNode.value) return 'design-node';
    if (selectedDesignEdge.value) return 'design-edge';
    return !props.selectedNodeId && !props.selectedEdge ? 'design-overview' : 'fallback';
  }
  return 'server';
});

watch(
  () => store.designMode,
  (designMode) => {
    if (designMode && store.workflows.length === 0) void store.refreshWorkflows();
  },
  { immediate: true },
);
watch(
  () => [selectedToolDesignNode.value !== null, store.tools.length] as const,
  async ([toolNodeSelected, toolCount]) => {
    if (!toolNodeSelected || toolCount > 0 || attemptedToolLoad.value) return;
    attemptedToolLoad.value = true;
    loadingToolSchemas.value = true;
    try {
      await store.refreshTools();
    } finally {
      loadingToolSchemas.value = false;
    }
  },
  { immediate: true },
);

function formatNodeType(type?: PipelineNodeType): string {
  return type === 'sequentialThinking'
    ? 'Sequential Thinking'
    : type === 'prompt'
      ? 'Prompt'
        : type === 'tool'
          ? 'Tool'
          : 'Workflow';
}

function getProbe(agentName: string) {
  return capabilitiesStore.getProbe(agentName);
}

function formatAgentOptionLabel(agentName: string): string {
  const probe = getProbe(agentName);
  if (!probe) return agentName;
  if (probe.error) return `${agentName} ⚠`;
  if (probe.modes.length <= 2 && probe.modes.length > 0) return `${agentName} — ${probe.modes.join(', ')}`;
  if (probe.modes.length > 2) return `${agentName} (${probe.modes.length} modes)`;
  return probe.models.length ? `${agentName} (${probe.models.length} model${probe.models.length === 1 ? '' : 's'})` : agentName;
}

function capabilityBadges(agentName?: string | null): string[] {
  if (!agentName) return [];
  const probe = getProbe(agentName);
  if (!probe) return [];
  if (probe.error) return ['⚠ probe failed'];
  return [...probe.modes, ...(probe.models.length ? [probe.models.length === 1 ? probe.models[0] : `${probe.models.length} models`] : [])];
}

function getFieldValue(event: Event): string {
  return (event.target as HTMLInputElement | HTMLTextAreaElement).value;
}

function updateMaxConcurrency(event: Event): void {
  const value = Number((event.target as HTMLInputElement).value);
  store.maxConcurrency = Number.isFinite(value) ? Math.min(32, Math.max(1, value)) : 1;
}
function replaceNode(previousId: string, nextNode: PipelineNode): void {
  store.localDefinition = {
    nodes: store.localDefinition.nodes.map((node) => (node.id === previousId ? nextNode : node)),
    edges: store.localDefinition.edges.map((edge) => ({
      ...edge,
      source: edge.source === previousId ? nextNode.id : edge.source,
      target: edge.target === previousId ? nextNode.id : edge.target,
    })),
  };
}
function updateDesignNode(update: (node: PipelineNode) => PipelineNode): void {
  if (!selectedDesignNode.value) return;
  const nextNode = update(selectedDesignNode.value);
  if (nextNode === selectedDesignNode.value) return;
  replaceNode(selectedDesignNode.value.id, nextNode);
}
function updateEdgeType(type: PipelineEdge['type']): void {
  if (!props.selectedEdge) return;
  store.localDefinition = {
    nodes: [...store.localDefinition.nodes],
    edges: store.localDefinition.edges.map((edge) =>
      edge.source === props.selectedEdge?.source && edge.target === props.selectedEdge?.target ? { ...edge, type } : edge,
    ),
  };
}
function updateNodeId(event: Event): void {
  updateDesignNode((node) => ({ ...node, id: getFieldValue(event) }));
}
function updateWorkflowField(event: Event, field: 'workflowId' | 'acpClient' | 'prompt'): void {
  const value = getFieldValue(event);
  const resolved = field === 'workflowId' ? value : (value || undefined);
  updateDesignNode((node) => (isWorkflowNode(node) ? { ...node, [field]: resolved } : node));
}
function updatePromptField(event: Event, field: 'agent' | 'prompt'): void {
  updateDesignNode((node) => (isPromptNode(node) ? { ...node, [field]: getFieldValue(event) } : node));
}
function updateThinkingField(event: Event, field: 'topic' | 'agent' | 'analysisType'): void {
  updateDesignNode((node) =>
    isSequentialThinkingNode(node) ? { ...node, [field]: getFieldValue(event) } : node,
  );
}
function updateToolName(event: Event): void {
  const nextToolName = getFieldValue(event);
  updateDesignNode((node) => {
    if (!isToolNode(node)) return node;
    if (nextToolName === node.toolName) return node;
    const { toolArgs: _toolArgs, ...rest } = node;
    return { ...rest, toolName: nextToolName };
  });
}
function updateToolArgs(data: Record<string, unknown>): void {
  const normalizedToolArgs = Object.keys(data).length > 0 ? data : undefined;
  updateDesignNode((node) => {
    if (!isToolNode(node)) return node;
    if (!normalizedToolArgs) {
      if (node.toolArgs == null) return node;
      const { toolArgs, ...rest } = node;
      return rest;
    }
    if (JSON.stringify(node.toolArgs ?? EMPTY_TOOL_ARGS) === JSON.stringify(normalizedToolArgs)) {
      return node;
    }
    return { ...node, toolArgs: normalizedToolArgs };
  });
}
function updateTotalThoughts(event: Event): void {
  const raw = getFieldValue(event);
  const totalThoughts = raw === '' ? undefined : Number(raw);
  if (raw !== '' && !Number.isFinite(totalThoughts)) return;
  updateDesignNode((node) =>
    isSequentialThinkingNode(node) ? { ...node, totalThoughts } : node,
  );
}
</script>

<template>
  <aside class="inspector" aria-label="Pipeline inspector">
    <section v-if="inspectorMode === 'design-overview'" class="panel">
      <h2>Pipeline Settings</h2>
      <label class="field"><span>Max concurrency</span><input class="field-input" :value="store.maxConcurrency" type="number" min="1" max="32" @input="updateMaxConcurrency" /></label>
      <div class="stats-grid">
        <div class="stat-card"><span class="stat-label">Nodes</span><strong>{{ store.localDefinition.nodes.length }}</strong></div>
        <div class="stat-card"><span class="stat-label">Edges</span><strong>{{ store.localDefinition.edges.length }}</strong></div>
      </div>
    </section>

    <section v-else-if="inspectorMode === 'design-node' && selectedDesignNode" class="panel">
      <h2>Node</h2>
      <label class="field"><span>ID</span><input class="field-input" :value="selectedDesignNode.id" type="text" @input="updateNodeId" /></label>
      <div class="summary-row"><span>Type</span><strong class="type-pill">{{ formatNodeType(getNodeType(selectedDesignNode)) }}</strong></div>

      <template v-if="selectedWorkflowDesignNode">
        <label class="field"><span>Workflow ID</span><input class="field-input" :value="selectedWorkflowDesignNode.workflowId" :list="workflowListId" type="text" @input="updateWorkflowField($event, 'workflowId')" /><datalist :id="workflowListId"><option v-for="workflow in store.workflows" :key="workflow" :value="workflow" /></datalist></label>
        <label class="field"><span>ACP client</span><select class="field-input" :value="selectedWorkflowDesignNode.acpClient ?? ''" @change="updateWorkflowField($event, 'acpClient')"><option value="">Select agent…</option><option v-for="name in configStore.agentNames" :key="name" :value="name">{{ formatAgentOptionLabel(name) }}</option></select><div v-if="capabilityBadges(selectedWorkflowDesignNode.acpClient).length" class="agent-capabilities"><span v-for="badge in capabilityBadges(selectedWorkflowDesignNode.acpClient)" :key="badge" class="agent-capability">{{ badge }}</span></div></label>
        <label class="field"><span>Prompt</span><textarea class="field-textarea" :value="selectedWorkflowDesignNode.prompt ?? ''" rows="5" placeholder="Optional per-node prompt" @input="updateWorkflowField($event, 'prompt')" /></label>
      </template>

      <template v-else-if="selectedPromptDesignNode">
        <label class="field"><span>Agent</span><select class="field-input" :value="selectedPromptDesignNode.agent" @change="updatePromptField($event, 'agent')"><option value="" disabled>Select agent…</option><option v-for="name in configStore.agentNames" :key="name" :value="name">{{ formatAgentOptionLabel(name) }}</option></select><div v-if="capabilityBadges(selectedPromptDesignNode.agent).length" class="agent-capabilities"><span v-for="badge in capabilityBadges(selectedPromptDesignNode.agent)" :key="badge" class="agent-capability">{{ badge }}</span></div></label>
        <label class="field"><span>Prompt template</span><textarea class="field-textarea" :value="selectedPromptDesignNode.prompt" rows="6" @input="updatePromptField($event, 'prompt')" /></label>
      </template>

      <template v-else-if="selectedThinkingDesignNode">
        <label class="field"><span>Topic</span><textarea class="field-textarea" :value="selectedThinkingDesignNode.topic" rows="5" @input="updateThinkingField($event, 'topic')" /></label>
        <label class="field"><span>Agent</span><select class="field-input" :value="selectedThinkingDesignNode.agent" @change="updateThinkingField($event, 'agent')"><option value="" disabled>Select agent…</option><option v-for="name in configStore.agentNames" :key="name" :value="name">{{ formatAgentOptionLabel(name) }}</option></select><div v-if="capabilityBadges(selectedThinkingDesignNode.agent).length" class="agent-capabilities"><span v-for="badge in capabilityBadges(selectedThinkingDesignNode.agent)" :key="badge" class="agent-capability">{{ badge }}</span></div></label>
        <label class="field"><span>Total thoughts</span><input class="field-input" :value="selectedThinkingDesignNode.totalThoughts ?? ''" type="number" min="1" @input="updateTotalThoughts" /></label>
        <label class="field"><span>Analysis type</span><input class="field-input" :value="selectedThinkingDesignNode.analysisType ?? ''" type="text" @input="updateThinkingField($event, 'analysisType')" /></label>
      </template>

      <template v-else-if="selectedToolDesignNode">
        <label class="field"><span>Tool name</span><select class="field-input" :value="selectedToolDesignNode.toolName" @change="updateToolName"><option value="" disabled>Select tool…</option><optgroup v-for="cat in toolOptions" :key="cat.group" :label="cat.group"><option v-for="tool in cat.tools" :key="tool.name" :value="tool.name">{{ tool.name }}</option></optgroup></select><small v-if="loadingToolSchemas">Loading tool schema…</small></label>
        <div class="field">
          <span>Tool args</span>
          <SchemaForm :schema="selectedToolSchema" :data="selectedToolArgsData" @change="updateToolArgs" />
        </div>
      </template>
    </section>

    <section v-else-if="inspectorMode === 'design-edge' && selectedDesignEdge" class="panel">
      <h2>Edge</h2>
      <div class="summary-row"><span>Path</span><strong>{{ selectedDesignEdge.source }} → {{ selectedDesignEdge.target }}</strong></div>
      <fieldset class="field-group">
        <legend>Edge type</legend>
        <label class="radio-option"><input :checked="(selectedDesignEdge.type ?? 'success') === 'success'" type="radio" name="edge-type" @change="updateEdgeType('success')" /><span>Success</span></label>
        <label class="radio-option"><input :checked="selectedDesignEdge.type === 'always'" type="radio" name="edge-type" @change="updateEdgeType('always')" /><span>Always</span></label>
      </fieldset>
      <EdgeMappingEditor
        :edge="selectedDesignEdge"
        :source-node="selectedEdgeSourceNode"
        :target-node="selectedEdgeTargetNode"
      />
    </section>

    <PipelineInspectorServer
      v-else-if="inspectorMode === 'server'"
      :selected-node-id="selectedNodeId"
      :selected-edge="selectedEdge"
    />
    <section v-else class="panel empty-state"><h2>Inspector</h2><p>Select a node or edge to inspect</p></section>
  </aside>
</template>

<style scoped>
.inspector { height: 100%; background: var(--bg-sidebar); color: var(--text-primary); border-left: 1px solid var(--border-color); overflow-y: auto; }
.panel { padding: 1rem; display: flex; flex-direction: column; gap: 0.875rem; }
.panel h2 { font-size: var(--text-xl); font-weight: 600; }
.field,.field-group { display: flex; flex-direction: column; gap: 0.375rem; color: var(--text-secondary); font-size: var(--text-sm); }
.agent-capabilities { display: flex; flex-wrap: wrap; gap: 0.35rem; }
.agent-capability { display: inline-flex; align-items: center; padding: 0.15rem 0.45rem; border: 1px solid var(--border-color); border-radius: 999px; color: var(--text-secondary); background: var(--bg-main); font-size: var(--text-xs); }
.field-group { border: 1px solid var(--border-color); border-radius: 8px; padding: 0.75rem; background: var(--bg-main); }
.field-group legend { padding: 0 0.25rem; color: var(--text-primary); }
.summary-row,.radio-option,.stat-card { display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; }
.summary-row span,.stat-label { color: var(--text-secondary); }
.stats-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 0.75rem; }
.stat-card,.type-pill { padding: 0.875rem; border: 1px solid var(--border-color); border-radius: 8px; background: var(--bg-main); }
.type-pill { padding: 0.2rem 0.6rem; font-size: var(--text-sm); }
.radio-option { justify-content: flex-start; }
.empty-state { justify-content: center; min-height: 220px; }
.empty-state p { color: var(--text-secondary); }
</style>
