<script setup lang="ts">
import ChatToolCallItem from './ChatToolCallItem.vue';
import type { ToolCallInfo } from '../lib/types';

defineProps<{
  toolCalls: ToolCallInfo[];
  expandedToolCallIds: string[];
}>();

const emit = defineEmits<{ 'toggle-tool-call': [toolCallId: string] }>();
</script>

<template>
  <div class="flex flex-col gap-1 mb-3" role="region" aria-label="Tool calls">
    <ChatToolCallItem
      v-for="toolCall in toolCalls"
      :key="toolCall.toolCallId"
      :tool-call="toolCall"
      :is-expanded="expandedToolCallIds.includes(toolCall.toolCallId)"
      @toggle="emit('toggle-tool-call', toolCall.toolCallId)"
    />
  </div>
</template>
