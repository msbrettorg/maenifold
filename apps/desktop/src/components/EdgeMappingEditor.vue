<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { resolveToolDefinition } from '../lib/tool-categories';
import { usePipelineStore } from '../stores/pipeline';
import { useMcpToolDiscoveryStore } from '../stores/mcp-tool-discovery';
import { getNodeType, isToolNode, type PipelineEdge, type PipelineNode } from '../lib/pipeline-types';

interface MappingRow {
  id: string;
  param: string;
  path: string;
}

interface ToolParam {
  name: string;
  description?: string;
  required: boolean;
}

interface SchemaProperty {
  description?: string;
}

const props = defineProps<{
  edge: PipelineEdge;
  sourceNode: PipelineNode | null;
  targetNode: PipelineNode | null;
}>();

const store = usePipelineStore();
const discoveryStore = useMcpToolDiscoveryStore();
const loadingSchemas = ref(false);
const attemptedToolLoad = ref(false);
const manualRows = ref<MappingRow[]>([]);
const sourcePathListId = computed(() => `edge-mapping-${props.edge.source}-${props.edge.target}`);
const isToolTarget = computed(() => props.targetNode !== null && isToolNode(props.targetNode));
const targetToolName = computed(() =>
  props.targetNode !== null && isToolNode(props.targetNode) ? props.targetNode.toolName : null,
);
const targetTool = computed(() =>
  resolveToolDefinition(discoveryStore.allTools, targetToolName.value)
  ?? resolveToolDefinition(store.tools, targetToolName.value)
  ?? null,
);
const sourceOutput = computed(() => store.currentView?.nodes.find((node) => node.nodeId === props.edge.source)?.output);
const sourceOutputKeys = computed(() => {
  if (!sourceOutput.value) return [];

  try {
    const parsed = JSON.parse(sourceOutput.value) as unknown;
    return parsed && typeof parsed === 'object' && !Array.isArray(parsed) ? Object.keys(parsed) : [];
  } catch {
    return [];
  }
});
const targetParams = computed<ToolParam[]>(() => {
  const schema = targetTool.value?.inputSchema;
  if (!schema || typeof schema !== 'object' || Array.isArray(schema)) return [];

  const properties = (schema as { properties?: unknown }).properties;
  if (!properties || typeof properties !== 'object' || Array.isArray(properties)) return [];

  const required = new Set(
    Array.isArray((schema as { required?: unknown }).required)
      ? ((schema as { required?: unknown[] }).required ?? []).filter(
          (value): value is string => typeof value === 'string',
        )
      : [],
  );

  return Object.entries(properties as Record<string, unknown>).map(([name, value]) => ({
    name,
    description:
      value &&
      typeof value === 'object' &&
      !Array.isArray(value) &&
      typeof (value as SchemaProperty).description === 'string'
        ? (value as SchemaProperty).description
        : undefined,
    required: required.has(name),
  }));
});
const hasSourceOutput = computed(() => !!sourceOutput.value);
const hasMappings = computed(() => Object.keys(props.edge.mapping ?? {}).length > 0);
const targetLabel = computed(() => (props.targetNode ? getNodeType(props.targetNode) : 'node'));

watch(
  () => `${props.edge.source}->${props.edge.target}:${JSON.stringify(props.edge.mapping ?? {})}`,
  () => {
    manualRows.value = Object.entries(props.edge.mapping ?? {}).map(([param, path], index) => ({
      id: `${param}-${index}`,
      param,
      path,
    }));
  },
  { immediate: true },
);

watch(
  () => [isToolTarget.value, discoveryStore.allTools.length, store.tools.length] as const,
  async ([toolTarget, discoveryCount, pipelineCount]) => {
    if (!toolTarget || discoveryCount > 0 || pipelineCount > 0 || attemptedToolLoad.value) return;
    attemptedToolLoad.value = true;
    loadingSchemas.value = true;
    try {
      await discoveryStore.refreshTools();
      if (discoveryStore.allTools.length === 0) {
        await store.refreshTools();
      }
    } finally {
      loadingSchemas.value = false;
    }
  },
  { immediate: true },
);

function updateEdge(transform: (edge: PipelineEdge) => PipelineEdge): void {
  store.localDefinition = {
    nodes: [...store.localDefinition.nodes],
    edges: store.localDefinition.edges.map((edge) =>
      edge.source === props.edge.source && edge.target === props.edge.target ? transform(edge) : edge,
    ),
  };
}

function updateMappedField(param: string, value: string): void {
  const path = value.trim();
  updateEdge((edge) => {
    const mapping = { ...(edge.mapping ?? {}) };
    if (path) mapping[param] = path;
    else delete mapping[param];

    if (Object.keys(mapping).length === 0) {
      const { mapping: _mapping, ...rest } = edge;
      return rest;
    }

    return { ...edge, mapping };
  });
}

function commitManualRows(): void {
  const mapping = Object.fromEntries(
    manualRows.value
      .map((row) => [row.param.trim(), row.path.trim()] as const)
      .filter((entry): entry is [string, string] => entry[0].length > 0 && entry[1].length > 0),
  );
  updateEdge((edge) => {
    if (Object.keys(mapping).length === 0) {
      const { mapping: _mapping, ...rest } = edge;
      return rest;
    }
    return { ...edge, mapping };
  });
}

function addManualRow(): void {
  manualRows.value = [
    ...manualRows.value,
    { id: `${Date.now()}-${manualRows.value.length}`, param: '', path: '' },
  ];
}

function removeManualRow(id: string): void {
  manualRows.value = manualRows.value.filter((row) => row.id !== id);
  commitManualRows();
}
</script>

<template>
  <section class="mapping-editor" role="form" aria-label="Edge mapping">
    <div class="header">
      <h3>Field Mapping</h3>
      <p v-if="isToolTarget">Map source output paths into tool input parameters.</p>
      <p v-else>Predecessor context is prepended automatically — mapping is optional for ACP nodes.</p>
    </div>

    <p v-if="loadingSchemas" class="status">Loading tool schemas...</p>
    <p v-if="!hasSourceOutput" class="status">Source output will be available after execution</p>

    <template v-if="isToolTarget && targetParams.length > 0">
      <label v-for="param in targetParams" :key="param.name" class="mapping-row">
        <span class="param-label">
          <strong>{{ param.name }}</strong>
          <small>{{ param.required ? 'Required' : 'Optional' }}</small>
        </span>
        <input
          class="field-input"
          :list="sourceOutputKeys.length > 0 ? sourcePathListId : undefined"
          :placeholder="sourceOutputKeys.length > 0 ? 'Source dot-path' : 'e.g. results[0].title'"
          :value="edge.mapping?.[param.name] ?? ''"
          type="text"
          @input="updateMappedField(param.name, ($event.target as HTMLInputElement).value)"
        />
        <small v-if="param.description">{{ param.description }}</small>
      </label>
      <p v-if="!hasMappings" class="empty-copy">No field mappings — edge carries topology only</p>
    </template>

    <template v-else>
      <p v-if="isToolTarget && !loadingSchemas" class="status">Tool schema unavailable — enter parameter names manually</p>
      <div v-if="manualRows.length > 0" class="manual-list">
        <div v-for="row in manualRows" :key="row.id" class="manual-row">
          <input
            v-model="row.param"
            class="field-input"
            placeholder="Target parameter"
            type="text"
            @blur="commitManualRows"
          />
          <input
            v-model="row.path"
            class="field-input"
            :list="sourceOutputKeys.length > 0 ? sourcePathListId : undefined"
            :placeholder="sourceOutputKeys.length > 0 ? 'Source dot-path' : 'e.g. results[0].title'"
            type="text"
            @blur="commitManualRows"
          />
          <button type="button" class="btn btn-ghost" @click="removeManualRow(row.id)">Remove</button>
        </div>
      </div>
      <p v-else class="empty-copy">
        {{ hasMappings ? `Mappings configured for target ${targetLabel}` : 'No field mappings — edge carries topology only' }}
      </p>
      <button type="button" class="btn btn-ghost" @click="addManualRow">Add mapping</button>
    </template>

    <datalist v-if="sourceOutputKeys.length > 0" :id="sourcePathListId">
      <option v-for="path in sourceOutputKeys" :key="path" :value="path" />
    </datalist>
  </section>
</template>

<style scoped>
.mapping-editor { display: flex; flex-direction: column; gap: 0.75rem; padding-top: 0.5rem; border-top: 1px solid var(--border); }
.header { display: flex; flex-direction: column; gap: 0.25rem; }
.header h3 { font-size: var(--text-base); font-weight: var(--weight-title); }
.header p,.status,.empty-copy,.mapping-row small { color: var(--muted-foreground); font-size: var(--text-sm); }
.mapping-row,.manual-list { display: flex; flex-direction: column; gap: 0.5rem; }
.param-label,.manual-row { display: flex; align-items: center; gap: 0.5rem; justify-content: space-between; }
.manual-row { align-items: stretch; padding: 0.75rem; border: 1px solid var(--border); border-radius: 8px; background: var(--background); }
.manual-row .field-input { flex: 1; }
@media (max-width: 640px) { .manual-row { flex-direction: column; } }
</style>
