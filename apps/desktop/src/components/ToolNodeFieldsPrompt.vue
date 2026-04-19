<script setup lang="ts">
import type { PropType } from 'vue';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import type { AgentProbeConfigOption, AgentProbeResult } from '../lib/agent-probe-types';
import type { PipelineNodeAgentConfig } from '../lib/pipeline-types';
import type { SessionConfigSelectGroup, SessionConfigSelectOption } from '../lib/types';
import AgentProbePreview from './AgentProbePreview.vue';

function syntheticChangeEvent(value: unknown): Event {
  return { target: { value: String(value ?? '') } } as unknown as Event;
}

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
  <Label>
    <span>Agent Override</span>
    <Select :model-value="agentName" @update:model-value="(val) => updateAgentName(syntheticChangeEvent(val))">
      <SelectTrigger :class="selectFieldClass" data-testid="tool-agent-select">
        <SelectValue placeholder="Use tool default" />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="">Use tool default</SelectItem>
        <SelectItem v-for="name in agentNames" :key="name" :value="name">
          {{ formatAgentOptionLabel(name) }}
        </SelectItem>
      </SelectContent>
    </Select>
  </Label>

  <div v-if="agentName" class="tool-agent-panel">
    <AgentProbePreview compact :agent-name="agentName" :probe="selectedAgentProbe" :loading="isProbeLoading(agentName)" />
    <template v-if="readyProbe">
      <div v-if="readyProbe.modeOptions.length" class="agent-mode-badges">
        <Button
          v-for="mode in readyProbe.modeOptions"
          :key="mode.id"
          variant="outline"
          size="sm"
          class="mode-badge"
          :class="{ selected: getAgentModeValue(agentConfig, readyProbe) === mode.id }"
          :data-testid="`tool-mode-badge-${mode.id}`"
          @click="updateAgentMode(mode.id)"
        >
          {{ mode.name }}
        </Button>
      </div>
      <div class="tool-agent-grid">
        <Label v-if="readyProbe.modelOptions.length">
          <span>Model</span>
          <Select :model-value="getAgentModelValue(agentConfig, readyProbe)" @update:model-value="(val) => updateAgentModel(syntheticChangeEvent(val))">
            <SelectTrigger :class="selectFieldClass" data-testid="tool-model-select">
              <SelectValue placeholder="Select model" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem v-for="model in readyProbe.modelOptions" :key="model.modelId" :value="model.modelId">{{ model.name }}</SelectItem>
            </SelectContent>
          </Select>
        </Label>
        <Label v-if="readyProbe.thoughtLevelConfigOption">
          <span>{{ readyProbe.thoughtLevelConfigOption.name }}</span>
          <Select :model-value="getAgentThoughtLevelValue(agentConfig, readyProbe)" @update:model-value="(val) => updateAgentThoughtLevel(syntheticChangeEvent(val))">
            <SelectTrigger :class="selectFieldClass" data-testid="tool-thought-level-select">
              <SelectValue placeholder="Select thought level" />
            </SelectTrigger>
            <SelectContent>
              <template v-for="entry in readyProbe.thoughtLevelConfigOption.options ?? []" :key="isSessionConfigSelectGroup(entry) ? entry.group : entry.value">
                <SelectGroup v-if="isSessionConfigSelectGroup(entry)">
                  <SelectLabel>{{ entry.name }}</SelectLabel>
                  <SelectItem v-for="groupOption in entry.options" :key="groupOption.value" :value="groupOption.value">{{ groupOption.name }}</SelectItem>
                </SelectGroup>
                <SelectItem v-else :value="entry.value">{{ entry.name }}</SelectItem>
              </template>
            </SelectContent>
          </Select>
        </Label>
        <Label v-for="option in editableAgentConfigOptions" :key="option.id">
          <span>{{ option.name }}</span>
          <Select :model-value="getAgentConfigOptionValue(agentConfig, option)" @update:model-value="(val) => updateAgentConfigOption(option, syntheticChangeEvent(val))">
            <SelectTrigger :class="selectFieldClass" :data-testid="`tool-config-${option.id}`">
              <SelectValue placeholder="Select option" />
            </SelectTrigger>
            <SelectContent>
              <template v-for="entry in option.options" :key="isSessionConfigSelectGroup(entry) ? entry.group : entry.value">
                <SelectGroup v-if="isSessionConfigSelectGroup(entry)">
                  <SelectLabel>{{ entry.name }}</SelectLabel>
                  <SelectItem v-for="groupOption in entry.options" :key="groupOption.value" :value="groupOption.value">{{ groupOption.name }}</SelectItem>
                </SelectGroup>
                <SelectItem v-else :value="entry.value">{{ entry.name }}</SelectItem>
              </template>
            </SelectContent>
          </Select>
        </Label>
      </div>
    </template>
  </div>
</template>
