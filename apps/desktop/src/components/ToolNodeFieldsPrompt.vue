<script setup lang="ts">
import type { PropType } from 'vue';
import type { AgentProbeConfigOption, AgentProbeResult } from '../lib/agent-probe-types';
import type { PipelineNodeAgentConfig } from '../lib/pipeline-types';
import type { SessionConfigSelectGroup, SessionConfigSelectOption } from '../lib/types';
import AgentProbePreview from './AgentProbePreview.vue';

const selectFieldClass =
  'w-full bg-input text-foreground border border-border rounded-md px-3 py-2 text-caption outline-none focus:border-ring focus:ring-2 focus:ring-ring/20';

defineProps({
  agentName: { type: String, default: '' },
  agentConfig: { type: Object as PropType<PipelineNodeAgentConfig>, default: () => ({}) },
  agentNames: { type: Array as PropType<string[]>, required: true },
  selectedAgentProbe: { type: Object as PropType<AgentProbeResult | undefined>, default: undefined },
  readyProbe: { type: Object as PropType<AgentProbeResult | undefined>, default: undefined },
  editableAgentConfigOptions: { type: Array as PropType<AgentProbeConfigOption[]>, required: true },
  formatAgentOptionLabel: { type: Function as PropType<(agentName: string) => string>, required: true },
  isProbeLoading: { type: Function as PropType<(agentName: string) => boolean>, required: true },
  getAgentModeValue: { type: Function as PropType<(config: PipelineNodeAgentConfig | undefined, probe: AgentProbeResult | undefined) => string | undefined>, required: true },
  getAgentModelValue: { type: Function as PropType<(config: PipelineNodeAgentConfig | undefined, probe: AgentProbeResult | undefined) => string | undefined>, required: true },
  getAgentThoughtLevelValue: { type: Function as PropType<(config: PipelineNodeAgentConfig | undefined, probe: AgentProbeResult | undefined) => string | undefined>, required: true },
  getAgentConfigOptionValue: { type: Function as PropType<(config: PipelineNodeAgentConfig | undefined, option: AgentProbeConfigOption) => string | undefined>, required: true },
  isSessionConfigSelectGroup: { type: Function as PropType<(entry: SessionConfigSelectOption | SessionConfigSelectGroup) => entry is SessionConfigSelectGroup>, required: true },
  updateAgentName: { type: Function as PropType<(event: Event) => void>, required: true },
  updateAgentMode: { type: Function as PropType<(modeId: string) => void>, required: true },
  updateAgentModel: { type: Function as PropType<(event: Event) => void>, required: true },
  updateAgentThoughtLevel: { type: Function as PropType<(event: Event) => void>, required: true },
  updateAgentConfigOption: { type: Function as PropType<(option: AgentProbeConfigOption, event: Event) => void>, required: true },
});
</script>

<template>
  <label>
    <span>Agent Override</span>
    <select :class="selectFieldClass" data-testid="tool-agent-select" :value="agentName" @change="updateAgentName">
      <option value="">Use tool default</option>
      <option v-for="name in agentNames" :key="name" :value="name">
        {{ formatAgentOptionLabel(name) }}
      </option>
    </select>
  </label>

  <div v-if="agentName" class="tool-agent-panel">
    <AgentProbePreview compact :agent-name="agentName" :probe="selectedAgentProbe" :loading="isProbeLoading(agentName)" />
    <template v-if="readyProbe">
      <div v-if="readyProbe.modeOptions.length" class="agent-mode-badges">
        <button
          v-for="mode in readyProbe.modeOptions"
          :key="mode.id"
          type="button"
          class="mode-badge"
          :class="{ selected: getAgentModeValue(agentConfig, readyProbe) === mode.id }"
          :data-testid="`tool-mode-badge-${mode.id}`"
          @click="updateAgentMode(mode.id)"
        >
          {{ mode.name }}
        </button>
      </div>
      <div class="tool-agent-grid">
        <label v-if="readyProbe.modelOptions.length">
          <span>Model</span>
          <select :class="selectFieldClass" data-testid="tool-model-select" :value="getAgentModelValue(agentConfig, readyProbe)" @change="updateAgentModel">
            <option v-for="model in readyProbe.modelOptions" :key="model.modelId" :value="model.modelId">{{ model.name }}</option>
          </select>
        </label>
        <label v-if="readyProbe.thoughtLevelConfigOption">
          <span>{{ readyProbe.thoughtLevelConfigOption.name }}</span>
          <select :class="selectFieldClass" data-testid="tool-thought-level-select" :value="getAgentThoughtLevelValue(agentConfig, readyProbe)" @change="updateAgentThoughtLevel">
            <template v-for="entry in readyProbe.thoughtLevelConfigOption.options ?? []" :key="isSessionConfigSelectGroup(entry) ? entry.group : entry.value">
              <optgroup v-if="isSessionConfigSelectGroup(entry)" :label="entry.name">
                <option v-for="groupOption in entry.options" :key="groupOption.value" :value="groupOption.value">{{ groupOption.name }}</option>
              </optgroup>
              <option v-else :value="entry.value">{{ entry.name }}</option>
            </template>
          </select>
        </label>
        <label v-for="option in editableAgentConfigOptions" :key="option.id">
          <span>{{ option.name }}</span>
          <select :class="selectFieldClass" :data-testid="`tool-config-${option.id}`" :value="getAgentConfigOptionValue(agentConfig, option)" @change="updateAgentConfigOption(option, $event)">
            <template v-for="entry in option.options" :key="isSessionConfigSelectGroup(entry) ? entry.group : entry.value">
              <optgroup v-if="isSessionConfigSelectGroup(entry)" :label="entry.name">
                <option v-for="groupOption in entry.options" :key="groupOption.value" :value="groupOption.value">{{ groupOption.name }}</option>
              </optgroup>
              <option v-else :value="entry.value">{{ entry.name }}</option>
            </template>
          </select>
        </label>
      </div>
    </template>
  </div>
</template>
