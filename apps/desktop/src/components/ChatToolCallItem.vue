<script setup lang="ts">
import { computed } from 'vue';
import type { Component } from 'vue';
import { Pencil, Trash2, Package, Search, Play, Globe, Wrench, Settings } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
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

function statusColorClass(status: string): string {
  switch (status) {
    case 'in_progress': return 'text-warning';
    case 'completed': return 'text-success';
    case 'failed': return 'text-danger';
    case 'pending':
    case 'cancelled':
    default: return 'text-text-secondary';
  }
}
</script>

<template>
  <div class="flex flex-col overflow-hidden rounded bg-surface-high text-sm" role="article" :aria-label="toolCallAriaLabel">
    <Button
      variant="ghost"
      class="flex items-center gap-2 w-full px-2 py-1 border-none bg-transparent text-inherit font-[inherit] text-left"
      :class="hasDetails(toolCall) ? 'cursor-pointer hover:bg-surface-highest' : ''"
      :aria-expanded="hasDetails(toolCall) ? isExpanded : undefined"
      :aria-controls="hasDetails(toolCall) ? detailsId : undefined"
      @click="hasDetails(toolCall) && emit('toggle')"
    >
      <span class="text-sm"><component :is="toolIcon(toolCall.kind)" class="size-4" :stroke-width="1.5" /></span>
      <span class="font-medium text-text-primary">{{ toolCall.title }}</span>
      <span v-if="locationLabel(toolCall)" class="flex-1 text-text-secondary text-xs whitespace-nowrap overflow-hidden text-ellipsis">{{ locationLabel(toolCall) }}</span>
      <span :class="['text-xs', statusColorClass(toolCall.status)]"><component :is="statusIcon(toolCall.status)" class="size-4" :stroke-width="1.5" /></span>
      <span v-if="hasDetails(toolCall)" class="text-text-secondary text-xs">{{ isExpanded ? '▲' : '▼' }}</span>
    </Button>
    <ChatToolCallDetails
      v-if="hasDetails(toolCall) && isExpanded"
      :id="detailsId"
      :tool-call="toolCall"
      :expanded="isExpanded"
    />
  </div>
</template>
