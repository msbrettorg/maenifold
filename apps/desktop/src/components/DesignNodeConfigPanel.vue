<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { computed, ref, watch } from 'vue';
import type {
  EdgeConditionEquals,
  PipelineEdge,
  PipelineNode,
  PipelineNodeType,
  PromptNode,
  SequentialThinkingNode,
  WorkflowNode,
} from '../lib/pipeline-types';
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
const NONE_OPTION = '__none__';

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
const supportsAdoptConfig = computed(() =>
  selectedDesignNode.value
    ? getNodeType(selectedDesignNode.value) !== 'tool'
    : false,
);
const selectedAdoptableNode = computed(() =>
  selectedDesignNode.value ? getAdoptableNode(selectedDesignNode.value) : null,
);
const roleAssetOptions = computed(() => store.roleAssets);
const colorAssetOptions = computed(() => store.colorAssets);
const perspectiveAssetOptions = computed(() => store.perspectiveAssets);
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

    if (
      node
      && getNodeType(node) !== 'tool'
      && roleAssetOptions.value.length === 0
      && colorAssetOptions.value.length === 0
      && perspectiveAssetOptions.value.length === 0
    ) {
      await store.refreshAdoptAssets();
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

function normalizeOptionalSelectValue(value: unknown): string | undefined {
  const nextValue = String(value ?? '').trim();
  if (nextValue.length === 0 || nextValue === NONE_OPTION) {
    return undefined;
  }
  return nextValue;
}

function getAdoptableNode(node: PipelineNode): WorkflowNode | PromptNode | SequentialThinkingNode | null {
  const nodeType = getNodeType(node);
  return nodeType === 'tool'
    ? null
    : node as WorkflowNode | PromptNode | SequentialThinkingNode;
}

function updateNodeAdoptField(field: 'role' | 'color' | 'perspective', value: unknown): void {
  const selectedValue = normalizeOptionalSelectValue(value);
  updateDesignNode((node) => {
    const adoptableNode = getAdoptableNode(node);
    if (!adoptableNode) {
      return node;
    }

    const nextAdopt = { ...(adoptableNode.adopt ?? {}) };
    if (selectedValue) {
      nextAdopt[field] = selectedValue;
    } else {
      delete nextAdopt[field];
    }

    const { adopt: _adopt, ...rest } = adoptableNode;
    return Object.keys(nextAdopt).length > 0
      ? { ...rest, adopt: nextAdopt }
      : rest;
  });
}

function updateNodeRequiresRole(value: unknown): void {
  const selectedValue = normalizeOptionalSelectValue(value);
  updateDesignNode((node) => {
    const adoptableNode = getAdoptableNode(node);
    if (!adoptableNode) {
      return node;
    }
    const { requiresRole: _requiresRole, ...rest } = adoptableNode;
    return selectedValue
      ? { ...rest, requiresRole: selectedValue }
      : rest;
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
  <aside class="pipeline-inspector" aria-label="Node configuration">
    <section v-if="inspectorMode === 'design-overview'" class="pipeline-inspector__panel">
      <h2>Pipeline settings</h2>
      <label class="pipeline-inspector__field">
        <span>Max concurrency</span>
        <Input
          :model-value="store.maxConcurrency"
          type="number"
          min="1"
          max="32"
          @update:model-value="updateMaxConcurrency"
        />
      </label>
      <div class="pipeline-inspector__stats-grid">
        <div class="pipeline-inspector__stat-card">
          <span class="pipeline-inspector__stat-label">Nodes</span>
          <strong>{{ store.localDefinition.nodes.length }}</strong>
        </div>
        <div class="pipeline-inspector__stat-card">
          <span class="pipeline-inspector__stat-label">Edges</span>
          <strong>{{ store.localDefinition.edges.length }}</strong>
        </div>
      </div>
    </section>

    <section
      v-else-if="inspectorMode === 'design-node' && selectedDesignNode"
      class="pipeline-inspector__panel"
    >
      <h2>{{ selectedNodeLabel || selectedDesignNode.id || 'Node' }}</h2>
      <label class="pipeline-inspector__field">
        <span>Label</span>
        <Input
          :model-value="selectedNodeLabel"
          type="text"
          placeholder="Human-readable label"
          @update:model-value="updateNodeLabel"
        />
      </label>
      <label class="pipeline-inspector__field">
        <span>ID</span>
        <Input
          class="opacity-60 cursor-default select-text bg-background"
          :model-value="selectedDesignNode.id"
          type="text"
          readonly
        />
      </label>
      <div class="pipeline-inspector__summary-row">
        <span>Type</span>
        <strong class="pipeline-inspector__type-pill">{{ formatNodeType(getNodeType(selectedDesignNode)) }}</strong>
      </div>

      <SchemaForm
        v-if="selectedNodeSchema"
        :schema="selectedNodeSchema"
        :model-value="nodeFormValue"
        aria-label="Node configuration form"
        @update:model-value="updateNodeFormValue"
      />

      <p v-if="nodeFormError" class="pipeline-inspector__field-hint pipeline-inspector__field-hint--warning">
        {{ nodeFormError }}
      </p>

      <Card
        v-if="supportsAdoptConfig"
        class="border-border bg-[var(--surface-container)]"
      >
        <CardHeader class="space-y-1.5">
          <CardTitle class="font-display text-[var(--text-sm)] text-[var(--color-text-primary)]">Adopt configuration</CardTitle>
          <CardDescription class="text-[var(--text-xs)] text-[var(--color-text-secondary)]">
            Apply role, color, and perspective before node dispatch.
          </CardDescription>
        </CardHeader>
        <CardContent class="grid gap-3">
          <label class="pipeline-inspector__field">
            <span>Role</span>
            <Select
              :model-value="selectedAdoptableNode?.adopt?.role ?? NONE_OPTION"
              @update:model-value="updateNodeAdoptField('role', $event)"
            >
              <SelectTrigger>
                <SelectValue placeholder="Select role" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem :value="NONE_OPTION">None</SelectItem>
                <SelectItem
                  v-for="option in roleAssetOptions"
                  :key="`role-${option.id}`"
                  :value="option.id"
                >
                  {{ option.id }}
                </SelectItem>
              </SelectContent>
            </Select>
          </label>
          <label class="pipeline-inspector__field">
            <span>Color</span>
            <Select
              :model-value="selectedAdoptableNode?.adopt?.color ?? NONE_OPTION"
              @update:model-value="updateNodeAdoptField('color', $event)"
            >
              <SelectTrigger>
                <SelectValue placeholder="Select color" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem :value="NONE_OPTION">None</SelectItem>
                <SelectItem
                  v-for="option in colorAssetOptions"
                  :key="`color-${option.id}`"
                  :value="option.id"
                >
                  {{ option.id }}
                </SelectItem>
              </SelectContent>
            </Select>
          </label>
          <label class="pipeline-inspector__field">
            <span>Perspective</span>
            <Select
              :model-value="selectedAdoptableNode?.adopt?.perspective ?? NONE_OPTION"
              @update:model-value="updateNodeAdoptField('perspective', $event)"
            >
              <SelectTrigger>
                <SelectValue placeholder="Select perspective" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem :value="NONE_OPTION">None</SelectItem>
                <SelectItem
                  v-for="option in perspectiveAssetOptions"
                  :key="`perspective-${option.id}`"
                  :value="option.id"
                >
                  {{ option.id }}
                </SelectItem>
              </SelectContent>
            </Select>
          </label>
          <label class="pipeline-inspector__field">
            <span>Requires role</span>
            <Select
              :model-value="selectedAdoptableNode?.requiresRole ?? NONE_OPTION"
              @update:model-value="updateNodeRequiresRole($event)"
            >
              <SelectTrigger>
                <SelectValue placeholder="Select required role" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem :value="NONE_OPTION">None</SelectItem>
                <SelectItem
                  v-for="option in roleAssetOptions"
                  :key="`requires-role-${option.id}`"
                  :value="option.id"
                >
                  {{ option.id }}
                </SelectItem>
              </SelectContent>
            </Select>
          </label>
        </CardContent>
      </Card>

      <label class="pipeline-inspector__field">
        <span>Working directory</span>
        <Input
          :model-value="selectedDesignNode.cwd ?? ''"
          type="text"
          placeholder="/absolute/path"
          @update:model-value="updateNodeCwd"
        />
        <small
          v-if="getCwdWarning(selectedDesignNode.cwd)"
          class="pipeline-inspector__field-hint pipeline-inspector__field-hint--warning"
        >
          {{ getCwdWarning(selectedDesignNode.cwd) }}
        </small>
      </label>
    </section>

    <section
      v-else-if="inspectorMode === 'design-edge' && selectedDesignEdge"
      class="pipeline-inspector__panel"
    >
      <h2>Edge</h2>
      <div class="pipeline-inspector__summary-row">
        <span>Path</span>
        <strong>{{ selectedDesignEdge.source }} → {{ selectedDesignEdge.target }}</strong>
      </div>
      <fieldset class="pipeline-inspector__field-group">
        <legend>Edge type</legend>
        <Select
          :model-value="selectedDesignEdge.type ?? 'success'"
          @update:model-value="(value) => updateEdgeType(String(value) as 'success' | 'always' | 'conditional')"
        >
          <SelectTrigger class="w-full">
            <SelectValue placeholder="Select edge type" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="success">Success</SelectItem>
            <SelectItem value="always">Always</SelectItem>
            <SelectItem value="conditional">Conditional</SelectItem>
          </SelectContent>
        </Select>
      </fieldset>
      <div v-if="isConditionalEdge" class="pipeline-inspector__field-group">
        <label class="pipeline-inspector__field">
          <span>Path</span>
          <Input
            :model-value="conditionalPath"
            type="text"
            placeholder="result.status"
            @update:model-value="updateConditionalPath"
          />
          <small
            v-if="conditionalPathError"
            class="pipeline-inspector__field-hint pipeline-inspector__field-hint--warning"
          >{{ conditionalPathError }}</small>
        </label>
        <label class="pipeline-inspector__field">
          <span>Equals type</span>
          <Select
            :model-value="conditionalEqualsType"
            @update:model-value="(value) => updateConditionalEqualsType(String(value))"
          >
            <SelectTrigger class="w-full">
              <SelectValue placeholder="Select equals type" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="string">String</SelectItem>
              <SelectItem value="number">Number</SelectItem>
              <SelectItem value="boolean">Boolean</SelectItem>
            </SelectContent>
          </Select>
        </label>
        <label class="pipeline-inspector__field">
          <span>Equals (JSON literal)</span>
          <Input
            :model-value="conditionalEqualsLiteral"
            type="text"
            :placeholder="conditionalEqualsPlaceholder()"
            @update:model-value="updateConditionalEqualsLiteral"
          />
          <small
            v-if="conditionalEqualsError"
            class="pipeline-inspector__field-hint pipeline-inspector__field-hint--warning"
          >{{ conditionalEqualsError }}</small>
        </label>
      </div>
      <EdgeMappingEditor
        :edge="selectedDesignEdge"
        :source-node="selectedEdgeSourceNode"
        :target-node="selectedEdgeTargetNode"
      />
    </section>

    <section v-else class="pipeline-inspector__panel pipeline-inspector__empty-state">
      <h2>Inspector</h2>
      <p>Select a node or edge to inspect</p>
    </section>
  </aside>
</template>
