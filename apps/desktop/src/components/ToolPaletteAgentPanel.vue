<script setup lang="ts">
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
import AgentProbePreview from './AgentProbePreview.vue';
import type { PipelineNodeAgentConfig } from '../lib/pipeline-types';
import type { AgentProbeResult, AgentProbeConfigOption } from '../lib/agent-probe-types-defs';
import type { SessionConfigSelectGroup, SessionConfigSelectOption } from '../lib/types';

const props = defineProps<{
  agentName: string;
  agentConfig: PipelineNodeAgentConfig;
  probe: AgentProbeResult | undefined;
  loading: boolean;
  ready: AgentProbeResult | undefined;
  selectFieldClass: string;
  panelClass: string;
  flexRowClass: string;
  getAgentModeValue: (config: PipelineNodeAgentConfig | undefined, ready: AgentProbeResult | undefined) => string;
  getAgentModelValue: (config: PipelineNodeAgentConfig | undefined, ready: AgentProbeResult | undefined) => string;
  getAgentThoughtLevelValue: (config: PipelineNodeAgentConfig | undefined, ready: AgentProbeResult | undefined) => string;
  getEditableAgentConfigOptions: (ready: AgentProbeResult | undefined) => AgentProbeConfigOption[];
  getAgentConfigOptionValue: (config: PipelineNodeAgentConfig | undefined, option: AgentProbeConfigOption) => string;
  isSessionConfigSelectGroup: (entry: SessionConfigSelectOption | SessionConfigSelectGroup) => entry is SessionConfigSelectGroup;
  syntheticChangeEvent: (value: unknown) => Event;
}>();

const emit = defineEmits<{
  'update:mode': [id: string];
  'update:model': [event: Event];
  'update:thoughtLevel': [event: Event];
  'update:configOption': [option: AgentProbeConfigOption, event: Event];
}>();
</script>

<template>
  <div :class="panelClass">
    <AgentProbePreview compact :agent-name="agentName" :probe="probe" :loading="loading" />
    <template v-if="ready">
      <div v-if="ready.modeOptions.length" :class="flexRowClass">
        <Button
          v-for="mode in ready.modeOptions"
          :key="mode.id"
          variant="outline"
          size="sm"
          :class="[
            'inline-flex items-center px-1.5 py-0.5 border border-border-subtle rounded text-xs bg-bg-app text-text-secondary cursor-pointer h-auto',
            getAgentModeValue(agentConfig, ready) === mode.id ? 'border-brand ring-1 ring-brand text-text-primary' : '',
          ]"
          :data-testid="`tool-mode-badge-${mode.id}`"
          @click="emit('update:mode', mode.id)"
        >{{ mode.name }}</Button>
      </div>
      <div class="grid gap-3 [grid-template-columns:repeat(auto-fit,minmax(11rem,1fr))]">
        <Label v-if="ready.modelOptions.length">
          <span>Model</span>
          <Select
            :model-value="getAgentModelValue(agentConfig, ready)"
            @update:model-value="(val) => emit('update:model', syntheticChangeEvent(val))"
          >
            <SelectTrigger :class="selectFieldClass" data-testid="tool-model-select">
              <SelectValue placeholder="Select model" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem v-for="model in ready.modelOptions" :key="model.modelId" :value="model.modelId">{{ model.name }}</SelectItem>
            </SelectContent>
          </Select>
        </Label>
        <Label v-if="ready.thoughtLevelConfigOption">
          <span>{{ ready.thoughtLevelConfigOption.name }}</span>
          <Select
            :model-value="getAgentThoughtLevelValue(agentConfig, ready)"
            @update:model-value="(val) => emit('update:thoughtLevel', syntheticChangeEvent(val))"
          >
            <SelectTrigger :class="selectFieldClass" data-testid="tool-thought-level-select">
              <SelectValue placeholder="Select thought level" />
            </SelectTrigger>
            <SelectContent>
              <template
                v-for="entry in ready.thoughtLevelConfigOption.options ?? []"
                :key="isSessionConfigSelectGroup(entry) ? entry.group : (entry as { value: string }).value"
              >
                <SelectGroup v-if="isSessionConfigSelectGroup(entry)">
                  <SelectLabel>{{ entry.name }}</SelectLabel>
                  <SelectItem v-for="groupOption in entry.options" :key="groupOption.value" :value="groupOption.value">{{ groupOption.name }}</SelectItem>
                </SelectGroup>
                <SelectItem v-else :value="(entry as { value: string }).value">{{ (entry as { name: string }).name }}</SelectItem>
              </template>
            </SelectContent>
          </Select>
        </Label>
        <Label v-for="option in getEditableAgentConfigOptions(ready)" :key="option.id">
          <span>{{ option.name }}</span>
          <Select
            :model-value="getAgentConfigOptionValue(agentConfig, option)"
            @update:model-value="(val) => emit('update:configOption', option, syntheticChangeEvent(val))"
          >
            <SelectTrigger :class="selectFieldClass" :data-testid="`tool-config-${option.id}`">
              <SelectValue placeholder="Select option" />
            </SelectTrigger>
            <SelectContent>
              <template
                v-for="entry in option.options"
                :key="isSessionConfigSelectGroup(entry) ? entry.group : (entry as { value: string }).value"
              >
                <SelectGroup v-if="isSessionConfigSelectGroup(entry)">
                  <SelectLabel>{{ entry.name }}</SelectLabel>
                  <SelectItem v-for="groupOption in entry.options" :key="groupOption.value" :value="groupOption.value">{{ groupOption.name }}</SelectItem>
                </SelectGroup>
                <SelectItem v-else :value="(entry as { value: string }).value">{{ (entry as { name: string }).name }}</SelectItem>
              </template>
            </SelectContent>
          </Select>
        </Label>
      </div>
    </template>
  </div>
</template>
