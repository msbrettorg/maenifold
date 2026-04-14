<script setup lang="ts">
import { computed } from 'vue';
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
import { isPromptNode } from '../lib/pipeline-types';
import { useAgentCapabilitiesStore } from '../stores/agent-capabilities';
import { useConfigStore } from '../stores/config';
import { usePipelineStore } from '../stores/pipeline';

const props = defineProps<{ nodeId: string }>();

const store = usePipelineStore();
const configStore = useConfigStore();
const capabilitiesStore = useAgentCapabilitiesStore();

const selectedNode = computed(() => {
  const node = store.localDefinition.nodes.find((entry) => entry.id === props.nodeId);
  return node && isPromptNode(node) ? node : null;
});
const selectedProbe = computed(() =>
  selectedNode.value?.agent ? capabilitiesStore.getProbe(selectedNode.value.agent) : undefined,
);
const readyProbe = useReadyAgentProbe(
  selectedProbe,
  computed(() => selectedNode.value?.agentConfig),
  (nextAgentConfig) => updateAgentConfig(nextAgentConfig),
);

function getFieldValue(event: Event): string {
  return (event.target as HTMLInputElement | HTMLTextAreaElement).value;
}

function formatAgentOptionLabel(agentName: string): string {
  return formatPipelineAgentOptionLabel(agentName, capabilitiesStore.getProbe(agentName));
}

function capabilityBadges(agentName?: string | null): string[] {
  return agentName ? getPipelineAgentCapabilityBadges(capabilitiesStore.getProbe(agentName)) : [];
}

function isProbeLoading(agentName?: string | null): boolean {
  return !!agentName && capabilitiesStore.probeStatus === 'probing' && !capabilitiesStore.getProbe(agentName);
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

function updateField(event: Event, field: 'agent' | 'prompt'): void {
  updateNode((node) => ({ ...node, [field]: getFieldValue(event) }));
}

function updateAgent(event: Event): void {
  const agent = getFieldValue(event);
  writeLastUsedAgent(agent);
  updateNode((node) => {
    const { agentConfig: _agentConfig, ...rest } = node;
    return { ...rest, agent };
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
  updateAgentConfig(setAgentModelValue(selectedNode.value?.agentConfig, getFieldValue(event)));
}

function updateThoughtLevel(event: Event): void {
  updateAgentConfig(setAgentThoughtLevelValue(selectedNode.value?.agentConfig, getFieldValue(event)));
}

function updateConfigOption(option: AgentProbeConfigOption, event: Event): void {
  updateAgentConfig(setAgentConfigOptionValue(selectedNode.value?.agentConfig, option, getFieldValue(event)));
}
</script>

<template>
  <template v-if="selectedNode">
    <label class="field">
      <span>Agent</span>
      <select class="field-input" :value="selectedNode.agent" data-testid="inspector-prompt-agent-select" @change="updateAgent">
        <option value="" disabled>Select agent…</option>
        <option v-for="name in configStore.agentNames" :key="name" :value="name">{{ formatAgentOptionLabel(name) }}</option>
      </select>
      <div v-if="capabilityBadges(selectedNode.agent).length" class="agent-capabilities">
        <span v-for="badge in capabilityBadges(selectedNode.agent)" :key="badge" class="agent-capability">{{ badge }}</span>
      </div>
    </label>
    <AgentProbePreview v-if="selectedNode.agent" compact :agent-name="selectedNode.agent" :probe="selectedProbe" :loading="isProbeLoading(selectedNode.agent)" />
    <div v-if="selectedNode.agent" class="agent-config-panel">
      <template v-if="readyProbe">
        <div v-if="readyProbe.modeOptions.length" class="agent-mode-badges">
          <button v-for="mode in readyProbe.modeOptions" :key="mode.id" type="button" class="mode-badge" :class="{ selected: getAgentModeValue(selectedNode.agentConfig, readyProbe) === mode.id }" @click="updateMode(mode.id)">{{ mode.name }}</button>
        </div>
        <div class="agent-config-grid">
          <label v-if="readyProbe.modelOptions.length" class="field"><span>Model</span><select class="field-input" :value="getAgentModelValue(selectedNode.agentConfig, readyProbe)" @change="updateModel"><option v-for="model in readyProbe.modelOptions" :key="model.modelId" :value="model.modelId">{{ model.name }}</option></select></label>
          <label v-if="readyProbe.thoughtLevelConfigOption" class="field"><span>{{ readyProbe.thoughtLevelConfigOption.name }}</span><select class="field-input" :value="getAgentThoughtLevelValue(selectedNode.agentConfig, readyProbe)" @change="updateThoughtLevel"><template v-for="entry in readyProbe.thoughtLevelConfigOption.options ?? []" :key="isSessionConfigSelectGroup(entry) ? entry.group : entry.value"><optgroup v-if="isSessionConfigSelectGroup(entry)" :label="entry.name"><option v-for="groupOption in entry.options" :key="groupOption.value" :value="groupOption.value">{{ groupOption.name }}</option></optgroup><option v-else :value="entry.value">{{ entry.name }}</option></template></select></label>
          <label v-for="option in getEditableAgentConfigOptions(readyProbe)" :key="option.id" class="field"><span>{{ option.name }}</span><select class="field-input" :value="getAgentConfigOptionValue(selectedNode.agentConfig, option)" @change="updateConfigOption(option, $event)"><template v-for="entry in option.options" :key="isSessionConfigSelectGroup(entry) ? entry.group : entry.value"><optgroup v-if="isSessionConfigSelectGroup(entry)" :label="entry.name"><option v-for="groupOption in entry.options" :key="groupOption.value" :value="groupOption.value">{{ groupOption.name }}</option></optgroup><option v-else :value="entry.value">{{ entry.name }}</option></template></select></label>
        </div>
      </template>
    </div>
    <label class="field"><span>Prompt template</span><textarea class="field-textarea" :value="selectedNode.prompt" rows="6" @input="updateField($event, 'prompt')" /></label>
  </template>
</template>
