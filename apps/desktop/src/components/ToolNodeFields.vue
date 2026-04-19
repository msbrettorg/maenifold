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

<style scoped>
:deep(.tool-palette),
:deep(.tool-agent-panel),
:deep(.tool-agent-grid),
:deep(.tool-palette__detail) { display: grid; gap: 0.75rem; }
:deep(.tool-agent-panel),
:deep(.tool-palette__detail) { padding: 0.8rem; border: 1px solid var(--color-border-subtle); border-radius: var(--radius); background: color-mix(in srgb, var(--color-bg-elevated) 72%, transparent); }
:deep(.tool-agent-grid) { grid-template-columns: repeat(auto-fit, minmax(11rem, 1fr)); }
:deep(.agent-mode-badges),
:deep(.tool-palette__option-tags),
:deep(.tool-palette__required-tags),
:deep(.tool-palette__detail-actions) { display: flex; flex-wrap: wrap; gap: 0.35rem; align-items: center; }
:deep(.mode-badge),
:deep(.tool-palette__tag),
:deep(.tool-palette__required-tag),
:deep(.tool-palette__server) { display: inline-flex; align-items: center; padding: 0.15rem 0.45rem; border: 1px solid var(--color-border-subtle); border-radius: 4px; font-size: var(--text-xs); }
:deep(.mode-badge) { background: var(--color-bg-app); color: var(--color-text-secondary); cursor: pointer; }
:deep(.mode-badge.selected),
:deep(.tool-palette__option.is-selected) { border-color: var(--color-brand); box-shadow: 0 0 0 1px var(--color-brand); }
:deep(.tool-palette__header),
:deep(.tool-palette__status-row),
:deep(.tool-palette__detail-head),
:deep(.tool-palette__detail-footer) { display: flex; justify-content: space-between; align-items: flex-start; gap: 0.75rem; }
:deep(.tool-palette__header p),
:deep(.tool-palette__detail-head p),
:deep(.tool-palette__status),
:deep(.tool-palette__option-desc),
:deep(.tool-palette__required-empty) { color: var(--color-text-secondary); font-size: var(--text-sm); }
:deep(.tool-palette__status--error),
:deep(.tool-palette__tag--warning) { color: var(--color-status-abandoned); }
:deep(.tool-palette__listbox) { display: grid; gap: 0.45rem; max-height: min(18rem, 38vh); padding: 0.1rem; overflow-y: auto; border-radius: var(--radius); outline: none; }
:deep(.tool-palette__option) { display: grid; gap: 0.35rem; padding: 0.7rem 0.8rem; border: 1px solid var(--color-border-subtle); border-radius: 0.75rem; background: var(--color-bg-app); color: var(--color-text-primary); text-align: left; cursor: pointer; }
:deep(.tool-palette__option:hover) { background: color-mix(in srgb, var(--color-text-primary) 4%, var(--color-bg-app)); }
:deep(.tool-palette__option:focus-visible),
:deep(.tool-palette__listbox:focus-visible) { outline: 2px solid var(--color-brand); outline-offset: 2px; }
:deep(.tool-palette__option-head) { display: flex; justify-content: space-between; align-items: flex-start; gap: 0.5rem; }
:deep(.tool-palette__option-desc) { display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
:deep(.tool-palette__server) { background: color-mix(in srgb, var(--color-text-primary) 8%, transparent); color: var(--color-text-secondary); }
:deep(.tool-palette__tag) { background: color-mix(in srgb, var(--color-text-primary) 6%, transparent); color: var(--color-text-secondary); }
:deep(.tool-palette__tag--warning) { border-color: color-mix(in srgb, var(--color-status-abandoned) 35%, var(--color-border-subtle)); }
:deep(.tool-palette__required) { display: grid; gap: 0.45rem; }
:deep(.tool-palette__required-tag) { background: color-mix(in srgb, var(--color-bg-app) 84%, transparent); }
:deep(mark) { padding: 0; background: color-mix(in srgb, var(--color-status-active) 24%, transparent); color: inherit; }
</style>
