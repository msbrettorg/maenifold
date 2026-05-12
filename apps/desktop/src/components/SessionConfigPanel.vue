<script setup lang="ts">
import { computed } from 'vue';
import type { AcceptableValue } from 'reka-ui';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { useAgentCapabilitiesStore } from '../stores/agent-capabilities';
import { useAgentSessionOverridesStore } from '../stores/agent-session-overrides';

const props = defineProps<{
  agentName: string;
  cwd: string;
  isReadOnly: boolean;
}>();

const capStore = useAgentCapabilitiesStore();
const overridesStore = useAgentSessionOverridesStore();

const probe = computed(() => capStore.getProbe(props.agentName));
const modelConfigOption = computed(() => probe.value?.modelConfigOption);
const modelConfigOptionId = computed(() => modelConfigOption.value?.id ?? 'model');
const modelOptions = computed(() => probe.value?.modelOptions ?? []);
const hasModelOptions = computed(() => modelOptions.value.length > 0);

const selectedModel = computed(() =>
  overridesStore.getSelectedOverride(props.agentName, modelConfigOptionId.value) ?? '',
);

function setModelOverride(value: string) {
  if (props.isReadOnly) return;
  overridesStore.setSelectedOverride(props.agentName, modelConfigOptionId.value, value);
}

function handleModelChange(value: AcceptableValue) {
  if (typeof value !== 'string') return;
  setModelOverride(value);
}

function handleModelInputChange(value: string | number) {
  setModelOverride(String(value));
}
</script>

<template>
  <div class="flex flex-col gap-3 px-4 py-3 bg-surface border-b border-border-subtle text-sm">
    <div class="flex flex-col gap-1">
      <span class="text-xs font-medium text-text-secondary uppercase tracking-wide">Working Directory</span>
      <span class="text-text-primary font-mono text-xs truncate" :title="cwd">{{ cwd || '/' }}</span>
    </div>

    <div v-if="modelConfigOption" class="flex flex-col gap-1">
      <span class="text-xs font-medium text-text-secondary uppercase tracking-wide">Model</span>
      <Select
        v-if="hasModelOptions"
        :model-value="selectedModel"
        :disabled="isReadOnly"
        @update:model-value="handleModelChange"
      >
        <SelectTrigger class="h-7 text-xs">
          <SelectValue placeholder="Default" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem
            v-for="m in modelOptions"
            :key="m.modelId"
            :value="m.modelId"
            class="text-xs"
          >
            {{ m.name || m.modelId }}
          </SelectItem>
        </SelectContent>
      </Select>
      <Input
        v-else
        :model-value="selectedModel"
        :disabled="isReadOnly"
        placeholder="Default"
        class="h-7 text-xs"
        @update:model-value="handleModelInputChange"
      />
    </div>

    <div class="flex flex-col gap-1">
      <span class="text-xs font-medium text-text-secondary uppercase tracking-wide">
        System Prompt
        <span class="ml-1 text-text-muted normal-case">(coming soon)</span>
      </span>
      <Textarea
        disabled
        placeholder="System prompt override is not yet supported"
        class="h-16 resize-none text-xs opacity-50"
      />
    </div>
  </div>
</template>
