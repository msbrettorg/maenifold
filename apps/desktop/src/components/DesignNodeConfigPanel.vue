<script setup lang="ts">
import { Input } from '@/components/ui/input';
import { computed, ref, watch } from 'vue';
import type { EdgeConditionEquals, PipelineEdge, PipelineNode, PipelineNodeType } from '../lib/pipeline-types';
import {
  cwdHasReasonablePathPrefix,
  getNodeType,
  normalizeNodeCwd,
} from '../lib/pipeline-types';
import {
  applyPipelineNodeSchemaFormValue,
  getPipelineNodeSchema,
  getPipelineNodeSchemaFormValue,
} from '../lib/pipeline-node-schemas';
import { useConfigStore } from '../stores/config';
import { usePipelineStore } from '../stores/pipeline';
import EdgeMappingEditor from './EdgeMappingEditor.vue';
import SchemaForm from './SchemaForm.vue';

const props = defineProps<{
  selectedNodeId: string | null;
  selectedEdge: { source: string; target: string } | null;
}>();

const store = usePipelineStore();
const configStore = useConfigStore();

const nodeFormValue = ref<Record<string, unknown>>({});
const nodeFormError = ref<string | null>(null);
const conditionalPath = ref('');
const conditionalEqualsLiteral = ref('""');
const conditionalEqualsType = ref<'string' | 'number' | 'boolean'>('string');
const conditionalEqualsError = ref<string | null>(null);

const selectedDesignNode = computed(() =>
  props.selectedNodeId
    ? store.localDefinition.nodes.find((node) => node.id === props.selectedNodeId) ?? null
    : null,
);
const selectedDesignEdge = computed(() =>
  props.selectedEdge
    ? store.localDefinition.edges.find(
        (edge) =>
          edge.source === props.selectedEdge?.source &&
          edge.target === props.selectedEdge?.target,
      ) ?? null
    : null,
);
const selectedNodeLabel = computed(() => selectedDesignNode.value?.displayLabel ?? '');
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
  if (selectedDesignNode.value) return 'design-node';
  if (selectedDesignEdge.value) return 'design-edge';
  return !props.selectedNodeId && !props.selectedEdge
    ? 'design-overview'
    : 'fallback';
});
const selectedNodeSchema = computed(() =>
  selectedDesignNode.value
    ? getPipelineNodeSchema(selectedDesignNode.value, {
        agentNames: configStore.agentNames,
        workflows: store.workflows,
      })
    : null,
);
const selectedEdgeType = computed(() => selectedDesignEdge.value?.type ?? 'success');
const isConditionalEdge = computed(() => selectedEdgeType.value === 'conditional');
const conditionalPathError = computed(() =>
  isConditionalEdge.value && conditionalPath.value.trim().length === 0
    ? 'Path is required for conditional edges.'
    : null,
);

watch(
  selectedDesignNode,
  (node) => {
    nodeFormValue.value = node ? getPipelineNodeSchemaFormValue(node) : {};
    nodeFormError.value = null;
  },
  { immediate: true },
);

watch(
  selectedDesignEdge,
  (edge) => {
    if (!edge || (edge.type ?? 'success') !== 'conditional') {
      conditionalPath.value = '';
      conditionalEqualsLiteral.value = '""';
      conditionalEqualsType.value = 'string';
      conditionalEqualsError.value = null;
      return;
    }

    const condition = edge.condition;
    const equalsValue = condition?.equals ?? '';
    conditionalPath.value = condition?.path ?? '';
    conditionalEqualsLiteral.value = JSON.stringify(equalsValue);
    conditionalEqualsType.value =
      typeof equalsValue === 'number'
        ? 'number'
        : typeof equalsValue === 'boolean'
          ? 'boolean'
          : 'string';
    conditionalEqualsError.value = null;
  },
  { immediate: true },
);

watch(
  () => [store.workflows.join('|'), configStore.agentNames.join('|')] as const,
  () => {
    if (!selectedDesignNode.value) {
      return;
    }

    const currentValue = nodeFormValue.value;
    const nextDefaults = getPipelineNodeSchemaFormValue(selectedDesignNode.value);
    nodeFormValue.value = {
      ...nextDefaults,
      ...currentValue,
    };
  },
);

watch(
  selectedDesignNode,
  async (node) => {
    if (node && getNodeType(node) === 'workflow' && store.workflows.length === 0) {
      await store.refreshWorkflows();
    }
  },
  { immediate: true },
);

function formatNodeType(type?: PipelineNodeType): string {
  return type === 'sequentialThinking'
    ? 'Sequential thinking'
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
  store.maxConcurrency = Number.isFinite(nextValue)
    ? Math.min(32, Math.max(1, nextValue))
    : 1;
}

function replaceNode(previousId: string, nextNode: PipelineNode): void {
  store.localDefinition = {
    ...store.localDefinition,
    nodes: store.localDefinition.nodes.map((node) =>
      node.id === previousId ? nextNode : node,
    ),
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
  if (nextNode !== selectedDesignNode.value) {
    replaceNode(selectedDesignNode.value.id, nextNode);
  }
}

function updateEdgeType(type: PipelineEdge['type']): void {
  if (!props.selectedEdge) return;
  conditionalPath.value = '';
  conditionalEqualsLiteral.value = '""';
  conditionalEqualsType.value = 'string';
  conditionalEqualsError.value = null;
  store.localDefinition = {
    ...store.localDefinition,
    nodes: [...store.localDefinition.nodes],
    edges: store.localDefinition.edges.map((edge) =>
      edge.source === props.selectedEdge?.source &&
      edge.target === props.selectedEdge?.target
        ? (() => {
            if (type !== 'conditional') {
              const { condition: _condition, ...rest } = edge;
              return { ...rest, type };
            }
            return {
              ...edge,
              type,
              condition: { path: '', equals: '' },
            };
          })()
        : edge,
    ),
  };
}

function parseEqualsLiteral(
  raw: string,
  expectedType: 'string' | 'number' | 'boolean',
): { ok: true; value: EdgeConditionEquals } | { ok: false; error: string } {
  try {
    const value = JSON.parse(raw) as unknown;
    if (expectedType === 'string' && typeof value === 'string') return { ok: true, value };
    if (expectedType === 'number' && typeof value === 'number' && Number.isFinite(value)) return { ok: true, value };
    if (expectedType === 'boolean' && typeof value === 'boolean') return { ok: true, value };
    return { ok: false, error: `Value must be a ${expectedType} JSON literal.` };
  } catch {
    return { ok: false, error: 'Invalid JSON literal.' };
  }
}

function updateConditionalEdge(
  update: (condition: { path: string; equals: EdgeConditionEquals }) => { path: string; equals: EdgeConditionEquals },
): void {
  if (!props.selectedEdge || selectedEdgeType.value !== 'conditional') return;
  store.localDefinition = {
    ...store.localDefinition,
    nodes: [...store.localDefinition.nodes],
    edges: store.localDefinition.edges.map((edge) => {
      if (edge.source !== props.selectedEdge?.source || edge.target !== props.selectedEdge?.target) {
        return edge;
      }
      const base = edge.condition ?? { path: '', equals: '' as EdgeConditionEquals };
      return { ...edge, type: 'conditional', condition: update(base) };
    }),
  };
}

function updateConditionalPath(value: string | number): void {
  conditionalPath.value = getFieldValue(value);
  updateConditionalEdge((condition) => ({ ...condition, path: conditionalPath.value.trim() }));
}

function updateConditionalEqualsLiteral(value: string | number): void {
  conditionalEqualsLiteral.value = getFieldValue(value);
  const parsed = parseEqualsLiteral(conditionalEqualsLiteral.value, conditionalEqualsType.value);
  if (!parsed.ok) {
    conditionalEqualsError.value = parsed.error;
    updateConditionalEdge((condition) => ({ ...condition, equals: '' }));
    return;
  }
  conditionalEqualsError.value = null;
  updateConditionalEdge((condition) => ({ ...condition, equals: parsed.value }));
}

function updateConditionalEqualsType(value: string | number): void {
  const nextType = getFieldValue(value);
  if (nextType !== 'string' && nextType !== 'number' && nextType !== 'boolean') return;
  conditionalEqualsType.value = nextType;
  updateConditionalEqualsLiteral(conditionalEqualsLiteral.value);
}

function conditionalEqualsPlaceholder(): string {
  if (conditionalEqualsType.value === 'string') return '"ok"';
  if (conditionalEqualsType.value === 'number') return '1';
  return 'true';
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

function updateNodeFormValue(nextValue: Record<string, unknown>): void {
  if (!selectedDesignNode.value) {
    return;
  }

  nodeFormValue.value = nextValue;

  try {
    const nextNode = applyPipelineNodeSchemaFormValue(
      selectedDesignNode.value,
      nextValue,
    );
    nodeFormError.value = null;
    updateDesignNode(() => nextNode);
  } catch (error) {
    nodeFormError.value =
      error instanceof Error ? error.message : 'Unable to update node configuration.';
  }
}
</script>

<template>
  <aside class="inspector" aria-label="Node configuration">
    <section v-if="inspectorMode === 'design-overview'" class="panel">
      <h2>Pipeline settings</h2>
      <label class="field">
        <span>Max concurrency</span>
        <Input
          :model-value="store.maxConcurrency"
          type="number"
          min="1"
          max="32"
          @update:model-value="updateMaxConcurrency"
        />
      </label>
      <div class="stats-grid">
        <div class="stat-card">
          <span class="stat-label">Nodes</span>
          <strong>{{ store.localDefinition.nodes.length }}</strong>
        </div>
        <div class="stat-card">
          <span class="stat-label">Edges</span>
          <strong>{{ store.localDefinition.edges.length }}</strong>
        </div>
      </div>
    </section>

    <section
      v-else-if="inspectorMode === 'design-node' && selectedDesignNode"
      class="panel"
    >
      <h2>{{ selectedNodeLabel || selectedDesignNode.id || 'Node' }}</h2>
      <label class="field">
        <span>Label</span>
        <Input
          :model-value="selectedNodeLabel"
          type="text"
          placeholder="Human-readable label"
          @update:model-value="updateNodeLabel"
        />
      </label>
      <label class="field">
        <span>ID</span>
        <Input
          class="opacity-60 cursor-default select-text bg-background"
          :model-value="selectedDesignNode.id"
          type="text"
          readonly
        />
      </label>
      <div class="summary-row">
        <span>Type</span>
        <strong class="type-pill">{{ formatNodeType(getNodeType(selectedDesignNode)) }}</strong>
      </div>

      <SchemaForm
        v-if="selectedNodeSchema"
        :schema="selectedNodeSchema"
        :model-value="nodeFormValue"
        aria-label="Node configuration form"
        @update:model-value="updateNodeFormValue"
      />

      <p v-if="nodeFormError" class="field-hint field-hint--warning">
        {{ nodeFormError }}
      </p>

      <label class="field">
        <span>Working directory</span>
        <Input
          :model-value="selectedDesignNode.cwd ?? ''"
          type="text"
          placeholder="/absolute/path"
          @update:model-value="updateNodeCwd"
        />
        <small
          v-if="getCwdWarning(selectedDesignNode.cwd)"
          class="field-hint field-hint--warning"
        >
          {{ getCwdWarning(selectedDesignNode.cwd) }}
        </small>
      </label>
    </section>

    <section
      v-else-if="inspectorMode === 'design-edge' && selectedDesignEdge"
      class="panel"
    >
      <h2>Edge</h2>
      <div class="summary-row">
        <span>Path</span>
        <strong>{{ selectedDesignEdge.source }} → {{ selectedDesignEdge.target }}</strong>
      </div>
      <fieldset class="field-group">
        <legend>Edge type</legend>
        <label class="radio-option">
          <input
            :checked="(selectedDesignEdge.type ?? 'success') === 'success'"
            type="radio"
            name="edge-type"
            @change="updateEdgeType('success')"
          >
          <span>Success</span>
        </label>
        <label class="radio-option">
          <input
            :checked="selectedDesignEdge.type === 'always'"
            type="radio"
            name="edge-type"
            @change="updateEdgeType('always')"
          >
          <span>Always</span>
        </label>
        <label class="radio-option">
          <input
            :checked="selectedDesignEdge.type === 'conditional'"
            type="radio"
            name="edge-type"
            @change="updateEdgeType('conditional')"
          >
          <span>Conditional</span>
        </label>
      </fieldset>
      <div v-if="isConditionalEdge" class="field-group">
        <label class="field">
          <span>Path</span>
          <Input
            :model-value="conditionalPath"
            type="text"
            placeholder="result.status"
            @update:model-value="updateConditionalPath"
          />
          <small v-if="conditionalPathError" class="field-hint field-hint--warning">{{ conditionalPathError }}</small>
        </label>
        <label class="field">
          <span>Equals type</span>
          <select
            :value="conditionalEqualsType"
            class="w-full rounded-[4px] bg-[var(--surface-high)] px-3 py-2 text-[var(--text-sm)] text-[var(--text)] outline-none ring-0"
            @change="updateConditionalEqualsType(($event.target as HTMLSelectElement).value)"
          >
            <option value="string">String</option>
            <option value="number">Number</option>
            <option value="boolean">Boolean</option>
          </select>
        </label>
        <label class="field">
          <span>Equals (JSON literal)</span>
          <Input
            :model-value="conditionalEqualsLiteral"
            type="text"
            :placeholder="conditionalEqualsPlaceholder()"
            @update:model-value="updateConditionalEqualsLiteral"
          />
          <small v-if="conditionalEqualsError" class="field-hint field-hint--warning">{{ conditionalEqualsError }}</small>
        </label>
      </div>
      <EdgeMappingEditor
        :edge="selectedDesignEdge"
        :source-node="selectedEdgeSourceNode"
        :target-node="selectedEdgeTargetNode"
      />
    </section>

    <section v-else class="panel empty-state">
      <h2>Inspector</h2>
      <p>Select a node or edge to inspect</p>
    </section>
  </aside>
</template>

<style scoped>
.inspector {
  height: 100%;
  background: var(--card);
  color: var(--foreground);
  border-left-width: 1px;
  border-left-style: solid;
  border-left-color: var(--border);
  overflow-y: auto;
}

.panel {
  padding: 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.875rem;
}

.panel h2 {
  font-size: var(--text-xl);
  font-weight: var(--weight-title);
}

.field,
.field-group {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
  color: var(--muted-foreground);
  font-size: var(--text-sm);
}

.field-hint {
  margin: 0;
  font-size: var(--text-xs);
}

.field-hint--warning {
  color: var(--color-status-active);
}

.field-group {
  border-width: 1px;
  border-style: solid;
  border-color: var(--border);
  border-radius: 4px;
  padding: 0.75rem;
  background: color-mix(in srgb, var(--background) 88%, transparent);
}

.field-group legend {
  padding: 0 0.25rem;
  color: var(--foreground);
}

.summary-row,
.radio-option,
.stat-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
}

.summary-row span,
.stat-label {
  color: var(--muted-foreground);
}

.summary-row strong {
  min-width: 0;
  word-break: break-word;
  overflow-wrap: break-word;
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.75rem;
}

.stat-card,
.type-pill {
  padding: 0.875rem;
  border-radius: 4px;
  background: color-mix(in srgb, var(--card) 92%, transparent);
}

.type-pill {
  padding: 0.2rem 0.6rem;
  font-size: var(--text-sm);
}

.radio-option {
  justify-content: flex-start;
}

.empty-state {
  justify-content: center;
  min-height: 220px;
}

.empty-state p {
  color: var(--muted-foreground);
}
</style>
