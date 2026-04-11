<script setup lang="ts">
import type { ToolCallInfo } from '../lib/types';
import ChatToolCallDetails from './ChatToolCallDetails.vue';

defineProps<{
  toolCall: ToolCallInfo;
  isExpanded: boolean;
}>();

const emit = defineEmits<{ toggle: [] }>();

function hasDetails(tc: ToolCallInfo): boolean {
  return Boolean(tc.content?.length || tc.rawInput || tc.rawOutput);
}

function locationLabel(tc: ToolCallInfo): string | null {
  const loc = tc.locations?.[0];
  if (!loc) return null;
  return loc.line != null ? `${loc.path}:${loc.line}` : loc.path;
}

function toolIcon(kind: string): string {
  switch (kind) {
    case 'read': return '📖';
    case 'edit': return '✏️';
    case 'delete': return '🗑️';
    case 'move': return '📦';
    case 'search': return '🔍';
    case 'execute': return '▶️';
    case 'think': return '💭';
    case 'fetch': return '🌐';
    default: return '🔧';
  }
}

function statusIcon(status: string): string {
  switch (status) {
    case 'pending': return '⏳';
    case 'in_progress': return '⚙️';
    case 'completed': return '✓';
    case 'failed': return '✗';
    case 'cancelled': return '⊘';
    default: return '';
  }
}
</script>

<template>
  <div :class="['tci-item', `tool-${toolCall.status}`]">
    <button
      type="button"
      class="tci-summary"
      :class="{ 'tci-summary-expandable': hasDetails(toolCall) }"
      @click="hasDetails(toolCall) && emit('toggle')"
    >
      <span class="tci-icon">{{ toolIcon(toolCall.kind) }}</span>
      <span class="tci-name">{{ toolCall.title }}</span>
      <span v-if="locationLabel(toolCall)" class="tci-location">{{ locationLabel(toolCall) }}</span>
      <span :class="['tci-status', `status-${toolCall.status}`]">{{ statusIcon(toolCall.status) }}</span>
      <span v-if="hasDetails(toolCall)" class="tci-chevron">{{ isExpanded ? '▲' : '▼' }}</span>
    </button>
    <ChatToolCallDetails v-if="hasDetails(toolCall) && isExpanded" :tool-call="toolCall" />
  </div>
</template>

<style scoped>
.tci-item {
  display: flex;
  flex-direction: column;
  border-radius: 4px;
  font-size: var(--text-sm);
  background: var(--bg-surface);
  border-left: 2px solid var(--border-default);
  overflow: hidden;
}
.tci-summary {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  width: 100%;
  padding: 0.375rem 0.625rem;
  border: none;
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
}
.tci-summary-expandable { cursor: pointer; }
.tci-summary-expandable:hover { background: var(--bg-surface); }
.tool-pending { border-left-color: var(--color-status-stuck); }
.tool-in_progress {
  border-left-color: var(--color-status-active);
  background: color-mix(in srgb, var(--color-status-active) 8%, var(--bg-surface));
}
.tool-completed {
  border-left-color: var(--color-status-completed);
  background: color-mix(in srgb, var(--color-status-completed) 8%, var(--bg-surface));
}
.tool-failed {
  border-left-color: var(--color-status-abandoned);
  background: color-mix(in srgb, var(--color-status-abandoned) 8%, var(--bg-surface));
}
.tool-cancelled {
  border-left-color: var(--color-status-pending);
  background: color-mix(in srgb, var(--color-status-pending) 8%, var(--bg-surface));
}
.tci-icon { font-size: var(--text-sm); }
.tci-name { font-weight: 500; color: var(--text-primary); }
.tci-location {
  flex: 1;
  color: var(--text-muted);
  font-size: var(--text-xs);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.tci-status { font-size: var(--text-xs); font-weight: 600; }
.tci-chevron { color: var(--text-muted); font-size: var(--text-xs); }
.status-pending { color: var(--color-status-stuck); }
.status-in_progress { color: var(--color-status-active); }
.status-completed { color: var(--color-status-completed); }
.status-failed { color: var(--color-status-abandoned); }
.status-cancelled { color: var(--color-status-pending); }
</style>
