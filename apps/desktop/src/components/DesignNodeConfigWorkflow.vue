<script setup lang="ts">
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { computed, onMounted } from 'vue';
import type { AgentProbeConfigOption } from '../lib/agent-probe-types';
import AgentProbePreview from './AgentProbePreview.vue';
import {
  formatPipelineAgentOptionLabel,
  getAgentConfigOptionValue,
  getAgentModeValue,
  getAgentModelValue,
  getAgentThoughtLevelValue,
  getEditableAgentConfigOptions,
  getPipelineAgentCapabilityBadges,
  isSessionConfigSelectGroup,
  setAgentConfigOptionValue,
  setAgentModeValue,
  setAgentModelValue,
  setAgentThoughtLevelValue,
} from '../composables/pipeline-node-form-helpers';
import { useReadyAgentProbe } from '../composables/useReadyAgentProbe';
import { writeLastUsedAgent } from '../lib/preferred-agent';
import type { PipelineNode, PipelineNodeAgentConfig } from '../lib/pipeline-types';
import { isWorkflowNode } from '../lib/pipeline-types';
import { useAgentCapabilitiesStore } from '../stores/agent-capabilities';
import { useConfigStore } from '../stores/config';
import { usePipelineStore } from '../stores/pipeline';

const props = withDefaults(defineProps<{ nodeId: string; workflowListId?: string }>(), {
  workflowListId: 'pipeline-inspector-workflows',
});

const store = usePipelineStore();
const configStore = useConfigStore();
const capabilitiesStore = useAgentCapabilitiesStore();
const nativeSelectClass =
  'w-full bg-input text-foreground border border-border rounded-md px-3 py-2 text-caption outline-none focus:border-ring focus:ring-2 focus:ring-ring/20';

const selectedNode = computed(() => {
  const node = store.localDefinition.nodes.find((entry) => entry.id === props.nodeId);
  return node && isWorkflowNode(node) ? node : null;
});
const selectedProbe = computed(() =>
  selectedNode.value?.acpClient ? capabilitiesStore.getProbe(selectedNode.value.acpClient) : undefined,
);
const readyProbe = useReadyAgentProbe(
  selectedProbe,
  computed(() => selectedNode.value?.agentConfig),
  (nextAgentConfig) => updateAgentConfig(nextAgentConfig),
);

onMounted(() => {
  if (store.workflows.length === 0) void store.refreshWorkflows();
});

function getFieldValue(value: string | number): string {
  return String(value);
}

function getSelectValue(event: Event): string {
  return (event.target as HTMLSelectElement).value;
}

function formatAgentOptionLabel(agentName: string): string {
  return formatPipelineAgentOptionLabel(agentName, capabilitiesStore.getProbe(agentName));
}

function capabilityBadges(agentName?: string | null): string[] {
  return agentName ? getPipelineAgentCapabilityBadges(capabilitiesStore.getProbe(agentName)) : [];
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

function updateNode(update: (node: NonNullable<typeof selectedNode.value>) => PipelineNode): void {
  if (!selectedNode.value) return;
  const nextNode = update(selectedNode.value);
  if (nextNode !== selectedNode.value) replaceNode(selectedNode.value.id, nextNode);
}

function updateTextField(value: string | number, field: 'workflowId' | 'prompt'): void {
  const resolvedValue = getFieldValue(value);
  const resolved = field === 'workflowId' ? resolvedValue : resolvedValue || undefined;
  updateNode((node) => ({ ...node, [field]: resolved }));
}

function updateAcpClient(event: Event): void {
  const value = getSelectValue(event);
  const resolved = value || undefined;
  if (value) writeLastUsedAgent(value);
  updateNode((node) => {
    const { model: _model, thoughtLevel: _thoughtLevel, agentConfig: _agentConfig, ...rest } = node;
    return { ...rest, acpClient: resolved };
  });
}

function updateAgentConfig(nextAgentConfig: PipelineNodeAgentConfig | undefined): void {
  updateNode((node) => {
    const { model: _model, thoughtLevel: _thoughtLevel, agentConfig: _agentConfig, ...rest } = node;
    return { ...rest, ...(nextAgentConfig ? { agentConfig: nextAgentConfig } : {}) };
  });
}

function updateMode(modeId: string): void {
  updateAgentConfig(setAgentModeValue(selectedNode.value?.agentConfig, modeId));
}

function updateModel(event: Event): void {
  updateAgentConfig(setAgentModelValue(selectedNode.value?.agentConfig, getSelectValue(event)));
}

function updateThoughtLevel(event: Event): void {
  updateAgentConfig(setAgentThoughtLevelValue(selectedNode.value?.agentConfig, getSelectValue(event)));
}

function updateConfigOption(option: AgentProbeConfigOption, event: Event): void {
  updateAgentConfig(setAgentConfigOptionValue(selectedNode.value?.agentConfig, option, getSelectValue(event)));
}
</script>

<template>
  <template v-if="selectedNode">
    <label class="field">
      <span>Workflow ID</span>
      <Input :model-value="selectedNode.workflowId" :list="workflowListId" type="text" @update:model-value="updateTextField($event, 'workflowId')" />
      <datalist :id="workflowListId"><option v-for="workflow in store.workflows" :key="workflow" :value="workflow" /></datalist>
    </label>
    <label class="field">
      <span>ACP client</span>
      <select :class="nativeSelectClass" :value="selectedNode.acpClient ?? ''" @change="updateAcpClient">
        <option value="">Select agent…</option>
        <option v-for="name in configStore.agentNames" :key="name" :value="name">{{ formatAgentOptionLabel(name) }}</option>
      </select>
      <div v-if="capabilityBadges(selectedNode.acpClient).length" class="agent-capabilities">
        <span v-for="badge in capabilityBadges(selectedNode.acpClient)" :key="badge" class="agent-capability">{{ badge }}</span>
      </div>
    </label>
    <AgentProbePreview v-if="selectedNode.acpClient" compact :agent-name="selectedNode.acpClient" :probe="selectedProbe" />
    <div v-if="selectedNode.acpClient" class="agent-config-panel">
      <template v-if="readyProbe">
        <div v-if="readyProbe.modeOptions.length" class="agent-mode-badges">
          <button v-for="mode in readyProbe.modeOptions" :key="mode.id" type="button" class="mode-badge" :class="{ selected: getAgentModeValue(selectedNode.agentConfig, readyProbe) === mode.id }" @click="updateMode(mode.id)">{{ mode.name }}</button>
        </div>
        <div class="agent-config-grid">
          <label v-if="readyProbe.modelOptions.length" class="field"><span>Model</span><select :class="nativeSelectClass" :value="getAgentModelValue(selectedNode.agentConfig, readyProbe)" @change="updateModel"><option v-for="model in readyProbe.modelOptions" :key="model.modelId" :value="model.modelId">{{ model.name }}</option></select></label>
          <label v-if="readyProbe.thoughtLevelConfigOption" class="field"><span>{{ readyProbe.thoughtLevelConfigOption.name }}</span><select :class="nativeSelectClass" :value="getAgentThoughtLevelValue(selectedNode.agentConfig, readyProbe)" @change="updateThoughtLevel"><template v-for="entry in readyProbe.thoughtLevelConfigOption.options ?? []" :key="isSessionConfigSelectGroup(entry) ? entry.group : entry.value"><optgroup v-if="isSessionConfigSelectGroup(entry)" :label="entry.name"><option v-for="groupOption in entry.options" :key="groupOption.value" :value="groupOption.value">{{ groupOption.name }}</option></optgroup><option v-else :value="entry.value">{{ entry.name }}</option></template></select></label>
          <label v-for="option in getEditableAgentConfigOptions(readyProbe)" :key="option.id" class="field"><span>{{ option.name }}</span><select :class="nativeSelectClass" :value="getAgentConfigOptionValue(selectedNode.agentConfig, option)" @change="updateConfigOption(option, $event)"><template v-for="entry in option.options" :key="isSessionConfigSelectGroup(entry) ? entry.group : entry.value"><optgroup v-if="isSessionConfigSelectGroup(entry)" :label="entry.name"><option v-for="groupOption in entry.options" :key="groupOption.value" :value="groupOption.value">{{ groupOption.name }}</option></optgroup><option v-else :value="entry.value">{{ entry.name }}</option></template></select></label>
        </div>
      </template>
    </div>
    <label class="field"><span>Prompt</span><Textarea :model-value="selectedNode.prompt ?? ''" rows="5" placeholder="Optional per-node prompt" @update:model-value="updateTextField($event, 'prompt')" /></label>
  </template>
</template>
