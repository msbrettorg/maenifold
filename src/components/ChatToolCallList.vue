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
  <div class="tool-calls-section">
    <ChatToolCallItem
      v-for="toolCall in toolCalls"
      :key="toolCall.toolCallId"
      :tool-call="toolCall"
      :is-expanded="expandedToolCallIds.includes(toolCall.toolCallId)"
      @toggle="emit('toggle-tool-call', toolCall.toolCallId)"
    />
  </div>
</template>

<style scoped>
.tool-calls-section { display: flex; flex-direction: column; gap: 0.25rem; margin-bottom: 0.75rem; }
</style>
