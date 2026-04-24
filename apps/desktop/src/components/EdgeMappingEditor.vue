<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
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
  <section class="flex flex-col gap-3 border-t border-border-subtle pt-2" role="form" aria-label="Edge mapping">
    <div class="flex flex-col gap-1">
      <h3 class="text-base font-semibold">Field Mapping</h3>
      <p v-if="isToolTarget" class="text-sm text-text-secondary">Map source output paths into tool input parameters.</p>
      <p v-else class="text-sm text-text-secondary">Predecessor context is prepended automatically — mapping is optional for ACP nodes.</p>
    </div>

    <p v-if="loadingSchemas" class="text-sm text-text-secondary">Loading tool schemas...</p>
    <p v-if="!hasSourceOutput" class="text-sm text-text-secondary">Source output will be available after execution</p>

    <template v-if="isToolTarget && targetParams.length > 0">
      <label v-for="param in targetParams" :key="param.name" class="flex flex-col gap-2 [&_small]:text-sm [&_small]:text-text-secondary">
        <span class="flex items-center gap-2 justify-between">
          <strong>{{ param.name }}</strong>
          <small>{{ param.required ? 'Required' : 'Optional' }}</small>
        </span>
        <Input
          :list="sourceOutputKeys.length > 0 ? sourcePathListId : undefined"
          :placeholder="sourceOutputKeys.length > 0 ? 'Source dot-path' : 'e.g. results[0].title'"
          :value="edge.mapping?.[param.name] ?? ''"
          type="text"
          @input="updateMappedField(param.name, ($event.target as HTMLInputElement).value)"
        />
        <small v-if="param.description">{{ param.description }}</small>
      </label>
      <p v-if="!hasMappings" class="text-sm text-text-secondary">No field mappings — edge carries topology only</p>
    </template>

    <template v-else>
      <p v-if="isToolTarget && !loadingSchemas" class="text-sm text-text-secondary">Tool schema unavailable — enter parameter names manually</p>
      <div v-if="manualRows.length > 0" class="flex flex-col gap-2">
        <Card v-for="row in manualRows" :key="row.id" size="sm" class="rounded-md py-0 shadow-none ring-0 border border-border-subtle bg-bg-app">
          <CardContent class="flex items-stretch justify-between gap-2 p-3 max-[640px]:flex-col">
            <Input
              v-model="row.param"
              class="flex-1"
              placeholder="Target parameter"
              type="text"
              @blur="commitManualRows"
            />
            <Input
              v-model="row.path"
              class="flex-1"
              :list="sourceOutputKeys.length > 0 ? sourcePathListId : undefined"
              :placeholder="sourceOutputKeys.length > 0 ? 'Source dot-path' : 'e.g. results[0].title'"
              type="text"
              @blur="commitManualRows"
            />
            <Button type="button" variant="ghost" @click="removeManualRow(row.id)">Remove</Button>
          </CardContent>
        </Card>
      </div>
      <p v-else class="text-sm text-text-secondary">
        {{ hasMappings ? `Mappings configured for target ${targetLabel}` : 'No field mappings — edge carries topology only' }}
      </p>
      <Button type="button" variant="ghost" @click="addManualRow">Add mapping</Button>
    </template>

    <datalist v-if="sourceOutputKeys.length > 0" :id="sourcePathListId">
      <option v-for="path in sourceOutputKeys" :key="path" :value="path" />
    </datalist>
  </section>
</template>
