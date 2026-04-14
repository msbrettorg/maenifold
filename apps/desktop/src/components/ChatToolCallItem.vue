<script setup lang="ts">
import { computed } from 'vue';
import type { ToolCallInfo } from '../lib/types';
import ChatToolCallDetails from './ChatToolCallDetails.vue';

const emit = defineEmits<{ toggle: [] }>();
const props = defineProps<{
  toolCall: ToolCallInfo;
  isExpanded: boolean;
}>();
const detailsId = computed(() => `tool-call-details-${props.toolCall.toolCallId}`);
const toolCallAriaLabel = computed(() => `${props.toolCall.title} tool call`);
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
  <div :class="['tci-item', `tool-${toolCall.status}`]" role="article" :aria-label="toolCallAriaLabel">
    <button
      type="button"
      class="tci-summary"
      :class="{ 'tci-summary-expandable': hasDetails(toolCall) }"
      :aria-expanded="hasDetails(toolCall) ? isExpanded : undefined"
      :aria-controls="hasDetails(toolCall) ? detailsId : undefined"
      @click="hasDetails(toolCall) && emit('toggle')"
    >
      <span class="tci-icon">{{ toolIcon(toolCall.kind) }}</span>
      <span class="tci-name">{{ toolCall.title }}</span>
      <span v-if="locationLabel(toolCall)" class="tci-location">{{ locationLabel(toolCall) }}</span>
      <span :class="['tci-status', `status-${toolCall.status}`]">{{ statusIcon(toolCall.status) }}</span>
      <span v-if="hasDetails(toolCall)" class="tci-chevron">{{ isExpanded ? '▲' : '▼' }}</span>
    </button>
    <ChatToolCallDetails
      v-if="hasDetails(toolCall) && isExpanded"
      :id="detailsId"
      :tool-call="toolCall"
      :expanded="isExpanded"
    />
  </div>
</template>

<style scoped>
.tci-item {
  display: flex;
  flex-direction: column;
  border-radius: 4px;
  font-size: var(--text-sm);
  background: #2a2a2a;
  overflow: hidden;
}
.tci-summary {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  width: 100%;
  padding: 0.25rem 0.5rem;
  border: none;
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
}
.tci-summary-expandable { cursor: pointer; }
.tci-summary-expandable:hover { background: #353534; }
.tci-icon { font-size: var(--text-sm); }
.tci-name { font-weight: var(--weight-display); color: var(--foreground); }
.tci-location {
  flex: 1;
  color: var(--muted-foreground);
  font-size: var(--text-xs);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.tci-status { font-size: var(--text-xs); }
.tci-chevron { color: var(--muted-foreground); font-size: var(--text-xs); }
.status-pending { color: #707070; }
.status-in_progress { color: #F59E0B; }
.status-completed { color: #4edea3; }
.status-failed { color: #ffb4ab; }
.status-cancelled { color: #707070; }
</style>
