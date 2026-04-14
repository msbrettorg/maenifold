<script setup lang="ts">
import { Input } from '@/components/ui/input';
import { computed } from 'vue';
import type { PipelineEdge, PipelineNode, PipelineNodeType } from '../lib/pipeline-types';
import {
  cwdHasReasonablePathPrefix,
  getNodeType,
  isPromptNode,
  isSequentialThinkingNode,
  isToolNode,
  isWorkflowNode,
  normalizeNodeCwd,
} from '../lib/pipeline-types';
import { usePipelineStore } from '../stores/pipeline';
import DesignNodeConfigPrompt from './DesignNodeConfigPrompt.vue';
import DesignNodeConfigThinking from './DesignNodeConfigThinking.vue';
import DesignNodeConfigTool from './DesignNodeConfigTool.vue';
import DesignNodeConfigWorkflow from './DesignNodeConfigWorkflow.vue';
import EdgeMappingEditor from './EdgeMappingEditor.vue';

const props = defineProps<{
  selectedNodeId: string | null;
  selectedEdge: { source: string; target: string } | null;
}>();

const store = usePipelineStore();
const workflowListId = 'pipeline-inspector-workflows';

const selectedDesignNode = computed(() =>
  props.selectedNodeId ? store.localDefinition.nodes.find((node) => node.id === props.selectedNodeId) ?? null : null,
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
  selectedDesignNode.value && isSequentialThinkingNode(selectedDesignNode.value) ? selectedDesignNode.value : null,
);
const selectedToolDesignNode = computed(() =>
  selectedDesignNode.value && isToolNode(selectedDesignNode.value) ? selectedDesignNode.value : null,
);
const selectedNodeLabel = computed(() => selectedDesignNode.value?.displayLabel ?? '');
const selectedEdgeSourceNode = computed(() =>
  selectedDesignEdge.value ? store.localDefinition.nodes.find((node) => node.id === selectedDesignEdge.value?.source) ?? null : null,
);
const selectedEdgeTargetNode = computed(() =>
  selectedDesignEdge.value ? store.localDefinition.nodes.find((node) => node.id === selectedDesignEdge.value?.target) ?? null : null,
);
const inspectorMode = computed(() => {
  if (selectedDesignNode.value) return 'design-node';
  if (selectedDesignEdge.value) return 'design-edge';
  return !props.selectedNodeId && !props.selectedEdge ? 'design-overview' : 'fallback';
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

function getFieldValue(value: string | number): string {
  return String(value);
}

function getCwdWarning(value: string | undefined): string | null {
  return value && !cwdHasReasonablePathPrefix(value)
    ? 'Path should usually start with /, ~/, ./, ../, or a drive letter.'
    : null;
}

function updateMaxConcurrency(value: string | number): void {
  const nextValue = Number(value);
  store.maxConcurrency = Number.isFinite(nextValue) ? Math.min(32, Math.max(1, nextValue)) : 1;
}

function replaceNode(previousId: string, nextNode: PipelineNode): void {
  store.localDefinition = {
    ...store.localDefinition,
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
  if (nextNode !== selectedDesignNode.value) replaceNode(selectedDesignNode.value.id, nextNode);
}

function updateEdgeType(type: PipelineEdge['type']): void {
  if (!props.selectedEdge) return;
  store.localDefinition = {
    ...store.localDefinition,
    nodes: [...store.localDefinition.nodes],
    edges: store.localDefinition.edges.map((edge) =>
      edge.source === props.selectedEdge?.source && edge.target === props.selectedEdge?.target ? { ...edge, type } : edge,
    ),
  };
}

function updateNodeLabel(value: string | number): void {
  const label = getFieldValue(value).trim();
  updateDesignNode((node) => {
    const { displayLabel: _displayLabel, ...rest } = node;
    return label ? { ...rest, displayLabel: label } : rest;
  });
}

function updateNodeCwd(value: string | number): void {
  const cwd = normalizeNodeCwd(getFieldValue(value));
  updateDesignNode((node) => {
    const { cwd: _cwd, ...rest } = node;
    return cwd ? { ...rest, cwd } : rest;
  });
}
</script>

<template>
  <aside class="inspector" aria-label="Node configuration">
    <section v-if="inspectorMode === 'design-overview'" class="panel">
      <h2>Pipeline Settings</h2>
      <label class="field">
        <span>Max concurrency</span>
        <Input :model-value="store.maxConcurrency" type="number" min="1" max="32" @update:model-value="updateMaxConcurrency" />
      </label>
      <div class="stats-grid">
        <div class="stat-card"><span class="stat-label">Nodes</span><strong>{{ store.localDefinition.nodes.length }}</strong></div>
        <div class="stat-card"><span class="stat-label">Edges</span><strong>{{ store.localDefinition.edges.length }}</strong></div>
      </div>
    </section>

    <section v-else-if="inspectorMode === 'design-node' && selectedDesignNode" class="panel">
      <h2>{{ selectedNodeLabel || selectedDesignNode.id || 'Node' }}</h2>
      <label class="field"><span>Label</span><Input :model-value="selectedNodeLabel" type="text" placeholder="Human-readable label" @update:model-value="updateNodeLabel" /></label>
      <label class="field"><span>ID</span><Input class="opacity-60 cursor-default select-text bg-background" :model-value="selectedDesignNode.id" type="text" readonly /></label>
      <div class="summary-row"><span>Type</span><strong class="type-pill">{{ formatNodeType(getNodeType(selectedDesignNode)) }}</strong></div>

      <DesignNodeConfigWorkflow v-if="selectedWorkflowDesignNode" :node-id="selectedWorkflowDesignNode.id" :workflow-list-id="workflowListId" />
      <DesignNodeConfigPrompt v-else-if="selectedPromptDesignNode" :node-id="selectedPromptDesignNode.id" />
      <DesignNodeConfigThinking v-else-if="selectedThinkingDesignNode" :node-id="selectedThinkingDesignNode.id" />
      <DesignNodeConfigTool v-else-if="selectedToolDesignNode" :node-id="selectedToolDesignNode.id" />

      <label v-if="!selectedToolDesignNode" class="field">
        <span>Working directory</span>
        <Input :model-value="selectedDesignNode.cwd ?? ''" type="text" placeholder="/absolute/path" @update:model-value="updateNodeCwd" />
        <small v-if="getCwdWarning(selectedDesignNode.cwd)" class="field-hint field-hint--warning">
          {{ getCwdWarning(selectedDesignNode.cwd) }}
        </small>
      </label>
    </section>

    <section v-else-if="inspectorMode === 'design-edge' && selectedDesignEdge" class="panel">
      <h2>Edge</h2>
      <div class="summary-row"><span>Path</span><strong>{{ selectedDesignEdge.source }} → {{ selectedDesignEdge.target }}</strong></div>
      <fieldset class="field-group">
        <legend>Edge type</legend>
        <label class="radio-option"><input :checked="(selectedDesignEdge.type ?? 'success') === 'success'" type="radio" name="edge-type" @change="updateEdgeType('success')" /><span>Success</span></label>
        <label class="radio-option"><input :checked="selectedDesignEdge.type === 'always'" type="radio" name="edge-type" @change="updateEdgeType('always')" /><span>Always</span></label>
      </fieldset>
      <EdgeMappingEditor :edge="selectedDesignEdge" :source-node="selectedEdgeSourceNode" :target-node="selectedEdgeTargetNode" />
    </section>

    <section v-else class="panel empty-state"><h2>Inspector</h2><p>Select a node or edge to inspect</p></section>
  </aside>
</template>

<style scoped>
.inspector { height: 100%; background: var(--card); color: var(--foreground); border-left: 1px solid var(--border); overflow-y: auto; }
.panel { padding: 1rem; display: flex; flex-direction: column; gap: 0.875rem; }
.panel h2 { font-size: var(--text-xl); font-weight: var(--weight-title); }
.panel :deep(.field),.panel :deep(.field-group) { display: flex; flex-direction: column; gap: 0.375rem; color: var(--muted-foreground); font-size: var(--text-sm); }
.panel :deep(.field-hint) { margin: 0; font-size: var(--text-xs); }
.panel :deep(.field-hint--warning) { color: var(--color-status-active); }
.panel :deep(.agent-capabilities) { display: flex; flex-wrap: wrap; gap: 0.35rem; }
.panel :deep(.agent-capability) { display: inline-flex; align-items: center; padding: 0.15rem 0.45rem; border: 1px solid var(--border); border-radius: 999px; color: var(--muted-foreground); background: var(--background); font-size: var(--text-xs); }
.panel :deep(.agent-config-panel) { display: grid; gap: 0.65rem; padding: 0.75rem; border: 1px solid var(--border); border-radius: 0.75rem; background: color-mix(in srgb, var(--background) 92%, transparent); }
.panel :deep(.agent-config-grid),.panel :deep(.agent-mode-badges) { display: flex; flex-wrap: wrap; gap: 0.65rem; }
.panel :deep(.agent-config-grid) { display: grid; grid-template-columns: repeat(auto-fit, minmax(11rem, 1fr)); }
.panel :deep(.mode-badge),.status-pill { display: inline-flex; align-items: center; padding: 0.15rem 0.45rem; border: 1px solid var(--border); border-radius: 999px; font-size: var(--text-xs); }
.panel :deep(.mode-badge) { background: var(--background); color: var(--muted-foreground); cursor: pointer; }
.panel :deep(.mode-badge.selected) { border-color: var(--ring); color: var(--foreground); background: color-mix(in srgb, var(--ring) 45%, transparent); }
.field-group { border: 1px solid var(--border); border-radius: 8px; padding: 0.75rem; background: var(--background); }
.field-group legend { padding: 0 0.25rem; color: var(--foreground); }
.summary-row,.radio-option,.stat-card { display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; }
.summary-row span,.stat-label { color: var(--muted-foreground); }
.summary-row strong,.panel :deep(.agent-capability) { min-width: 0; word-break: break-word; overflow-wrap: break-word; }
.stats-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 0.75rem; }
.stat-card,.type-pill { padding: 0.875rem; border: 1px solid var(--border); border-radius: 8px; background: var(--background); }
.type-pill { padding: 0.2rem 0.6rem; font-size: var(--text-sm); }
.radio-option { justify-content: flex-start; }
.empty-state { justify-content: center; min-height: 220px; }
.empty-state p { color: var(--muted-foreground); }
</style>
