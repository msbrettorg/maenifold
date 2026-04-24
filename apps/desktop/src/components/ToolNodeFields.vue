<script setup lang="ts">
import type { PipelineNodeAgentConfig } from '../lib/pipeline-types';
import ToolNodeFieldsTool from './ToolNodeFieldsTool.vue';

const props = withDefaults(defineProps<{
  active?: boolean;
  toolName: string;
  agentName?: string;
  agentConfig?: PipelineNodeAgentConfig;
  toolArgs: Record<string, unknown>;
  canSubmit?: boolean;
}>(), {
  active: false,
  agentName: '',
  agentConfig: () => ({}),
  canSubmit: false,
});

const emit = defineEmits<{
  'update:toolName': [value: string];
  'update:agentName': [value: string];
  'update:agentConfig': [value: PipelineNodeAgentConfig];
  'update:toolArgs': [value: Record<string, unknown>];
  validation: [state: { valid: boolean }];
  submitRequest: [];
}>();
</script>

<template>
  <ToolNodeFieldsTool
    v-bind="props"
    @update:toolName="emit('update:toolName', $event)"
    @update:agentName="emit('update:agentName', $event)"
    @update:agentConfig="emit('update:agentConfig', $event)"
    @update:toolArgs="emit('update:toolArgs', $event)"
    @validation="emit('validation', $event)"
    @submitRequest="emit('submitRequest')"
  />
</template>
