<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { Input } from '@/components/ui/input';
import { resolveToolDefinition } from '../lib/tool-categories';
import { usePipelineStore } from '../stores/pipeline';
import { useMcpToolDiscoveryStore } from '../stores/mcp-tool-discovery';
import type { PipelineEdge } from '../lib/pipeline-types-edge';
import { isToolNode, type PipelineNode } from '../lib/pipeline-types-node';

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
</script>

<template>
  <section class="flex flex-col gap-3 border-t border-border-subtle pt-2" role="form" aria-label="Edge mapping">
    <div class="flex flex-col gap-1">
      <h3 class="text-base font-semibold">Field Mapping</h3>
      <p v-if="isToolTarget" class="text-sm text-text-secondary">Map source output paths into tool input parameters.</p>
      <p v-else class="text-sm text-text-secondary">
        ACP target — typed parameter mapping is not available yet; predecessor context is prepended as text.
      </p>
    </div>

    <p v-if="loadingSchemas" class="text-sm text-text-secondary">Loading tool schemas...</p>
    <p v-if="!hasSourceOutput" class="text-sm text-text-secondary">Source output will be available after execution</p>

    <template v-if="isToolTarget">
      <template v-if="targetParams.length > 0">
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
      <p v-else-if="!loadingSchemas" class="text-sm text-text-secondary">No typed tool parameters loaded.</p>
    </template>
    <p v-else class="text-sm text-text-secondary">
      Typed bindings for ACP nodes are not available yet. ACP edges receive predecessor context as verbatim text prepend.
    </p>

    <datalist v-if="sourceOutputKeys.length > 0" :id="sourcePathListId">
      <option v-for="path in sourceOutputKeys" :key="path" :value="path" />
    </datalist>
  </section>
</template>
