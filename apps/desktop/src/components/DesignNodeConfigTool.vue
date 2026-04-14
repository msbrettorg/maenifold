<script setup lang="ts">
import { computed } from 'vue';
import ToolPaletteFields from './ToolPaletteFields.vue';
import { writeLastUsedAgent } from '../lib/preferred-agent';
import type { PipelineNode, PipelineNodeAgentConfig } from '../lib/pipeline-types';
import { isToolNode, normalizeNodeCwd } from '../lib/pipeline-types';
import { usePipelineStore } from '../stores/pipeline';

const EMPTY_TOOL_ARGS: Readonly<Record<string, unknown>> = Object.freeze({});

const props = defineProps<{ nodeId: string }>();

const store = usePipelineStore();

const selectedNode = computed(() => {
  const node = store.localDefinition.nodes.find((entry) => entry.id === props.nodeId);
  return node && isToolNode(node) ? node : null;
});
const selectedToolArgsData = computed<Record<string, unknown>>(() => selectedNode.value?.toolArgs ?? EMPTY_TOOL_ARGS);

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

function updateNode(update: (node: NonNullable<typeof selectedNode.value>) => PipelineNode): void {
  if (!selectedNode.value) return;
  const nextNode = update(selectedNode.value);
  if (nextNode !== selectedNode.value) replaceNode(selectedNode.value.id, nextNode);
}

function updateAgent(nextAgent: string): void {
  writeLastUsedAgent(nextAgent);
  updateNode((node) => {
    const { agentConfig: _agentConfig, ...rest } = node;
    if (!nextAgent) {
      const { agent: _agent, ...withoutAgent } = rest;
      return withoutAgent;
    }
    return { ...rest, agent: nextAgent };
  });
}

function updateAgentConfig(nextAgentConfig: PipelineNodeAgentConfig | undefined): void {
  updateNode((node) => {
    const { model: _model, thoughtLevel: _thoughtLevel, agentConfig: _agentConfig, ...rest } = node;
    return { ...rest, ...(nextAgentConfig ? { agentConfig: nextAgentConfig } : {}) };
  });
}

function updateToolName(nextToolName: string): void {
  updateNode((node) => {
    if (nextToolName === node.toolName) return node;
    const { toolArgs: _toolArgs, ...rest } = node;
    return { ...rest, toolName: nextToolName };
  });
}

function updateToolArgs(data: Record<string, unknown>): void {
  const normalizedToolArgs = Object.keys(data).length > 0 ? data : undefined;
  updateNode((node) => {
    if (!normalizedToolArgs) {
      if (node.toolArgs == null) return node;
      const { toolArgs: _toolArgs, ...rest } = node;
      return rest;
    }
    return JSON.stringify(node.toolArgs ?? EMPTY_TOOL_ARGS) === JSON.stringify(normalizedToolArgs)
      ? node
      : { ...node, toolArgs: normalizedToolArgs };
  });
}

function updateCwd(value: string | undefined): void {
  const cwd = normalizeNodeCwd(value);
  updateNode((node) => {
    const { cwd: _cwd, ...rest } = node;
    return cwd ? { ...rest, cwd } : rest;
  });
}
</script>

<template>
  <ToolPaletteFields
    v-if="selectedNode"
    :active="true"
    :tool-name="selectedNode.toolName"
    :agent-name="selectedNode.agent ?? ''"
    :agent-config="selectedNode.agentConfig ?? {}"
    :cwd="selectedNode.cwd ?? ''"
    :tool-args="selectedToolArgsData"
    @update:agent-name="updateAgent"
    @update:agent-config="updateAgentConfig"
    @update:cwd="updateCwd"
    @update:tool-name="updateToolName"
    @update:tool-args="updateToolArgs"
  />
</template>
