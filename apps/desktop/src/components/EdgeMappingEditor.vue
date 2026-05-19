<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { resolveToolDefinition } from '../lib/tool-categories';
import { usePipelineStore } from '../stores/pipeline';
import { useMcpToolDiscoveryStore } from '../stores/mcp-tool-discovery';
import type { EdgeMode, PipelineEdge } from '../lib/pipeline-types-edge';
import {
  isPromptNode,
  isSequentialThinkingNode,
  isToolNode,
  isWorkflowNode,
  type PromptNode,
  type PipelineNode,
  type SequentialThinkingNode,
  type WorkflowNode,
} from '../lib/pipeline-types-node';

interface ToolParam {
  name: string;
  description?: string;
  required: boolean;
}

interface SchemaProperty {
  description?: string;
}

type AcpNode = WorkflowNode | PromptNode | SequentialThinkingNode;

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
const currentEdge = computed(() =>
  store.localDefinition.edges.find(
    (edge) => edge.source === props.edge.source && edge.target === props.edge.target,
  ) ?? props.edge,
);
const edgeMode = computed<EdgeMode>(() => currentEdge.value.mode ?? 'context');
const isToolTarget = computed(() => props.targetNode !== null && isToolNode(props.targetNode));
const isAcpTarget = computed(() =>
  props.targetNode !== null &&
  (isWorkflowNode(props.targetNode) ||
    isPromptNode(props.targetNode) ||
    isSequentialThinkingNode(props.targetNode)),
);
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
  const outputSchema = props.sourceNode && typeof props.sourceNode === 'object'
    ? (props.sourceNode as { outputSchema?: unknown }).outputSchema
    : null;
  const schemaProperties = outputSchema &&
    typeof outputSchema === 'object' &&
    !Array.isArray(outputSchema)
    ? (outputSchema as { properties?: unknown }).properties
    : null;
  if (schemaProperties && typeof schemaProperties === 'object' && !Array.isArray(schemaProperties)) {
    return Object.keys(schemaProperties);
  }

  if (!sourceOutput.value) return [];

  try {
    const parsed = JSON.parse(sourceOutput.value) as unknown;
    return parsed && typeof parsed === 'object' && !Array.isArray(parsed) ? Object.keys(parsed) : [];
  } catch {
    return [];
  }
});
const toolParams = computed<ToolParam[]>(() => schemaToParams(targetTool.value?.inputSchema));
const acpParams = computed<ToolParam[]>(() => {
  const schema = getAcpInputSchema(props.targetNode);
  return schemaToParams(schema);
});
const targetParams = computed<ToolParam[]>(() => (isToolTarget.value ? toolParams.value : acpParams.value));
const hasInputSchema = computed(() =>
  isToolTarget.value ? toolParams.value.length > 0 : !!getAcpInputSchema(props.targetNode),
);
const showBindingForm = computed(() => edgeMode.value === 'binding' && hasInputSchema.value);
const showMissingSchema = computed(() => edgeMode.value === 'binding' && isAcpTarget.value && !hasInputSchema.value);
const hasSourceOutput = computed(() => sourceOutputKeys.value.length > 0 || !!sourceOutput.value);
const currentMapping = computed(() => currentEdge.value.mapping ?? {});
const mappingEntries = computed(() =>
  Object.entries(currentMapping.value).map(([targetPath, sourcePath]) => ({
    param: targetPath.startsWith('inputs.') ? targetPath.slice('inputs.'.length) : targetPath,
    sourcePath,
  })),
);
const hasBindings = computed(() => mappingEntries.value.some(({ sourcePath }) => !!sourcePath));

function schemaToParams(schema: unknown): ToolParam[] {
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
}

function isAcpNode(node: PipelineNode | null): node is AcpNode {
  return node !== null && (isWorkflowNode(node) || isPromptNode(node) || isSequentialThinkingNode(node));
}

function getAcpInputSchema(node: PipelineNode | null): Record<string, unknown> | undefined {
  return isAcpNode(node) ? node.inputSchema : undefined;
}

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

function updateMode(value: unknown): void {
  const mode = String(value ?? 'context');
  if (!['context', 'binding', 'none'].includes(mode)) return;
  updateEdge((edge) => ({ ...edge, mode: mode as EdgeMode }));
}

function bindingValue(param: string): string {
  return currentMapping.value[`inputs.${param}`] ?? '';
}

function updateBindingField(param: string, value: string): void {
  const path = value.trim();
  updateEdge((edge) => {
    const nextMapping = { ...(edge.mapping ?? {}) };
    const mappingKey = `inputs.${param}`;
    if (path) {
      nextMapping[mappingKey] = path;
    } else {
      delete nextMapping[mappingKey];
    }
    const { bindings: _bindings, mapping: _mapping, ...rest } = edge;

    return Object.keys(nextMapping).length > 0 ? { ...rest, mapping: nextMapping } : rest;
  });
}

function cleanupBindingField(param: string): void {
  updateEdge((edge) => {
    const nextMapping = Object.fromEntries(
      Object.entries(edge.mapping ?? {}).filter(
        ([targetPath, sourcePath]) => targetPath !== `inputs.${param}` || sourcePath.trim(),
      ),
    );
    const { bindings: _bindings, mapping: _mapping, ...rest } = edge;
    return Object.keys(nextMapping).length > 0 ? { ...rest, mapping: nextMapping } : rest;
  });
}
</script>

<template>
  <section class="flex flex-col gap-3 border-t border-border-subtle pt-2" role="form" aria-label="Edge mapping" :aria-invalid="showMissingSchema ? 'true' : undefined">
    <div class="flex flex-col gap-1">
      <h3 class="text-base font-semibold">Field Mapping</h3>
      <p v-if="edgeMode === 'binding'" class="text-sm text-text-secondary">Map source output paths into target input parameters.</p>
      <p v-else class="text-sm text-text-secondary">
        Choose how this edge moves data between nodes.
      </p>
    </div>

    <div class="grid gap-2">
      <Select :model-value="edgeMode" @update:model-value="updateMode">
        <SelectTrigger aria-label="Edge mode">
          <SelectValue placeholder="Select edge mode" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="context">Context</SelectItem>
          <SelectItem value="binding">Binding</SelectItem>
          <SelectItem value="none">None</SelectItem>
        </SelectContent>
      </Select>
    </div>

    <p v-if="loadingSchemas" class="text-sm text-text-secondary">Loading tool schemas...</p>
    <p v-if="edgeMode === 'binding' && !hasSourceOutput" class="text-sm text-text-secondary">Source output will be available after execution</p>

    <template v-if="edgeMode === 'context'">
      <p class="text-sm text-text-secondary">
        ACP edges receive predecessor context as verbatim text prepend.
      </p>
    </template>
    <template v-else-if="edgeMode === 'none'">
      <p class="text-sm text-text-secondary">Control-flow edge — no data crosses.</p>
    </template>
    <template v-else-if="showBindingForm">
      <template v-if="targetParams.length > 0">
        <label v-for="param in targetParams" :key="param.name" :data-testid="`${param.name}-binding-row`" class="flex flex-col gap-2 binding-row [&_small]:text-sm [&_small]:text-text-secondary">
          <span class="flex items-center gap-2 justify-between">
            <strong>{{ param.name }}</strong>
            <Badge variant="outline">{{ param.required ? 'Required' : 'Optional' }}</Badge>
          </span>
          <Input
            :name="param.name"
            :list="sourceOutputKeys.length > 0 ? sourcePathListId : undefined"
            :placeholder="sourceOutputKeys.length > 0 ? 'Source dot-path' : 'e.g. results[0].title'"
            :model-value="bindingValue(param.name)"
            type="text"
            @update:model-value="updateBindingField(param.name, String($event))"
            @blur="cleanupBindingField(param.name)"
          />
          <small v-if="param.description">{{ param.description }}</small>
        </label>
        <p v-if="!hasBindings" class="text-sm text-text-secondary">No field mappings — edge carries topology only</p>
      </template>
      <p v-else-if="!loadingSchemas" class="text-sm text-text-secondary">No typed tool parameters loaded.</p>
    </template>
    <template v-else-if="showMissingSchema">
      <Badge variant="destructive" aria-invalid="true">
        Target node must declare input_schema before bindings can be configured.
      </Badge>
      <Button type="submit" disabled class="sr-only">Bindings unavailable</Button>
    </template>

    <datalist v-if="sourceOutputKeys.length > 0" :id="sourcePathListId">
      <option v-for="path in sourceOutputKeys" :key="path" :value="path" />
    </datalist>
  </section>
</template>
