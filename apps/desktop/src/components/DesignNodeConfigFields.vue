<script setup lang="ts">
import { Input } from '@/components/ui/input';
import { computed, ref, watch } from 'vue';
import type { PipelineNode, PipelineNodeType } from '../lib/pipeline-types';
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
import DesignNodeConfigAgent from './DesignNodeConfigAgent.vue';
import SchemaForm from './SchemaForm.vue';

const props = defineProps<{
  selectedNodeId: string;
}>();

const store = usePipelineStore();
const configStore = useConfigStore();

const nodeFormValue = ref<Record<string, unknown>>({});
const nodeFormError = ref<string | null>(null);

const selectedDesignNode = computed(() =>
  store.localDefinition.nodes.find((node) => node.id === props.selectedNodeId) ?? null,
);
const selectedNodeLabel = computed(() => selectedDesignNode.value?.displayLabel ?? '');
const selectedNodeSchema = computed(() =>
  selectedDesignNode.value
    ? getPipelineNodeSchema(selectedDesignNode.value, {
        agentNames: configStore.agentNames,
        workflows: store.workflows,
      })
    : null,
);
const supportsAdoptConfig = computed(() =>
  selectedDesignNode.value
    ? getNodeType(selectedDesignNode.value) !== 'tool'
    : false,
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

function onAgentNodeUpdate(next: PipelineNode): void {
  updateDesignNode(() => next);
}
</script>

<template>
  <section v-if="selectedDesignNode" class="pipeline-inspector__panel">
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

    <DesignNodeConfigAgent
      v-if="supportsAdoptConfig"
      :node="selectedDesignNode"
      @update:node="onAgentNodeUpdate"
    />

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
</template>
