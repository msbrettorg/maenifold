<script setup lang="ts">
import { computed } from 'vue';
import type { Component } from 'vue';
import { Pencil, Trash2, Package, Search, Play, Globe, Wrench, Settings } from 'lucide-vue-next';
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

function toolIcon(kind: string): Component {
  switch (kind) {
    case 'edit': return Pencil;
    case 'delete': return Trash2;
    case 'move': return Package;
    case 'search': return Search;
    case 'execute': return Play;
    case 'fetch': return Globe;
    default: return Wrench;
  }
}

function statusIcon(status: string): Component {
  switch (status) {
    case 'in_progress': return Settings;
    default: return Wrench;
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
      <span class="tci-icon"><component :is="toolIcon(toolCall.kind)" class="size-4" :stroke-width="1.5" /></span>
      <span class="tci-name">{{ toolCall.title }}</span>
      <span v-if="locationLabel(toolCall)" class="tci-location">{{ locationLabel(toolCall) }}</span>
      <span :class="['tci-status', `status-${toolCall.status}`]"><component :is="statusIcon(toolCall.status)" class="size-4" :stroke-width="1.5" /></span>
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
  background: var(--surface-high);
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
.tci-summary-expandable:hover { background: var(--surface-highest); }
.tci-icon { font-size: var(--text-sm); }
.tci-name { font-weight: var(--weight-display); color: var(--color-text-primary); }
.tci-location {
  flex: 1;
  color: var(--color-text-secondary);
  font-size: var(--text-xs);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.tci-status { font-size: var(--text-xs); }
.tci-chevron { color: var(--color-text-secondary); font-size: var(--text-xs); }
.status-pending { color: var(--text-dim); }
.status-in_progress { color: var(--amber); }
.status-completed { color: var(--sage); }
.status-failed { color: var(--terracotta); }
.status-cancelled { color: var(--text-dim); }
</style>
