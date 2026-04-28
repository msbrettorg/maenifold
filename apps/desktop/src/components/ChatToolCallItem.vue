<script setup lang="ts">
import { computed } from 'vue';
import type { ToolCallInfo } from '../lib/types';
import McpAppFrame from './McpAppFrame.vue';
import ToolCallCard from './ToolCallCard.vue';
import ToolCallCardExtensions from './ToolCallCardExtensions.vue';

const emit = defineEmits<{ toggle: [] }>();
const props = defineProps<{
  toolCall: ToolCallInfo;
  isExpanded: boolean;
}>();
const detailsId = computed(() => `tool-call-details-${props.toolCall.toolCallId}`);
const toolCallAriaLabel = computed(() => `${props.toolCall.title} tool call`);
const hasAppUi = computed(() => Boolean(props.toolCall.appUi?.resourceUri));
function hasDetails(tc: ToolCallInfo): boolean {
  return Boolean(tc.content?.length || tc.rawInput || tc.rawOutput || tc.appUi?.resourceUri);
}
</script>

<template>
  <div class="flex flex-col overflow-hidden rounded bg-surface-high text-sm" role="article" :aria-label="toolCallAriaLabel">
    <ToolCallCard :tool-call="toolCall" :aria-controls="hasDetails(toolCall) ? detailsId : undefined">
      <template #details="{ toolCall: slottedToolCall }">
        <ToolCallCardExtensions
          v-if="hasDetails(slottedToolCall)"
          :id="detailsId"
          :tool-call="slottedToolCall"
          :expanded="isExpanded"
          @toggle="emit('toggle')"
        />
      </template>
    </ToolCallCard>
    <McpAppFrame
      v-if="hasAppUi && toolCall.status === 'completed'"
      :tool-call="toolCall"
      class="mx-2 mb-2"
    />
  </div>
</template>
