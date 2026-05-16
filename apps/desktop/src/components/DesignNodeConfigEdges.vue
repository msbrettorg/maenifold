<script setup lang="ts">
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { computed, ref, watch } from 'vue';
import type { EdgeConditionEquals, PipelineEdge } from '../lib/pipeline-types-edge';
import { usePipelineStore } from '../stores/pipeline';
import EdgeMappingEditor from './EdgeMappingEditor.vue';

const props = defineProps<{
  selectedEdge: { source: string; target: string };
}>();

const store = usePipelineStore();

const conditionalPath = ref('');
const conditionalEqualsLiteral = ref('""');
const conditionalEqualsType = ref<'string' | 'number' | 'boolean'>('string');
const conditionalEqualsError = ref<string | null>(null);

const selectedDesignEdge = computed(() =>
  store.localDefinition.edges.find(
    (edge) =>
      edge.source === props.selectedEdge.source &&
      edge.target === props.selectedEdge.target,
  ) ?? null,
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
const selectedEdgeType = computed(() => selectedDesignEdge.value?.type ?? 'success');
const isConditionalEdge = computed(() => selectedEdgeType.value === 'conditional');
const conditionalPathError = computed(() =>
  isConditionalEdge.value && conditionalPath.value.trim().length === 0
    ? 'Path is required for conditional edges.'
    : null,
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

function getFieldValue(value: string | number): string {
  return String(value);
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

function updateEdgeType(type: PipelineEdge['type']): void {
  conditionalPath.value = '';
  conditionalEqualsLiteral.value = '""';
  conditionalEqualsType.value = 'string';
  conditionalEqualsError.value = null;
  store.localDefinition = {
    ...store.localDefinition,
    nodes: [...store.localDefinition.nodes],
    edges: store.localDefinition.edges.map((edge) =>
      edge.source === props.selectedEdge.source &&
      edge.target === props.selectedEdge.target
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

function updateConditionalEdge(
  update: (condition: { path: string; equals: EdgeConditionEquals }) => { path: string; equals: EdgeConditionEquals },
): void {
  if (selectedEdgeType.value !== 'conditional') return;
  store.localDefinition = {
    ...store.localDefinition,
    nodes: [...store.localDefinition.nodes],
    edges: store.localDefinition.edges.map((edge) => {
      if (edge.source !== props.selectedEdge.source || edge.target !== props.selectedEdge.target) {
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
</script>

<template>
  <section v-if="selectedDesignEdge" class="pipeline-inspector__panel">
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
</template>
