<script setup lang="ts">
import { computed } from 'vue';
import DOMPurify from 'dompurify';
import { marked } from 'marked';
import ChatContentBlocks from './ChatContentBlocks.vue';
import ChatThoughtSection from './ChatThoughtSection.vue';
import ChatToolCallList from './ChatToolCallList.vue';
import type { ChatMessage } from '../lib/types';

const props = defineProps<{
  message: ChatMessage;
  expandedThoughtIds: string[];
  expandedToolCallIds: string[];
}>();

const emit = defineEmits<{
  'toggle-thought': [messageId: string];
  'toggle-tool-call': [toolCallId: string];
}>();

const toolCalls = computed(() => props.message.toolCalls ?? []);
const blocks = computed(() => props.message.blocks ?? []);
const legacyContentWithDOMPurify = computed(() =>
  props.message.content && !blocks.value.some((block) => block.type === 'text')
    ? renderMarkdown(props.message.content)
    : null
);
const isToolOnlyAssistantMessage = computed(() =>
  props.message.role === 'assistant' &&
  !props.message.content &&
  !blocks.value.length &&
  toolCalls.value.length > 0
);

function renderMarkdown(content: string): string {
  const parsed = marked.parse(content, { async: false });
  return DOMPurify.sanitize(typeof parsed === 'string' ? parsed : '');
}
</script>

<template>
  <div :class="['message', `message-${message.role}`, { 'message-tool-only': isToolOnlyAssistantMessage }]">
    <div class="message-header">
      <span class="role">{{ message.role === 'user' ? 'You' : 'Assistant' }}</span>
    </div>

    <ChatThoughtSection
      v-if="message.thought && message.role === 'assistant'"
      :message-id="message.id"
      :thought="message.thought"
      :expanded="expandedThoughtIds.includes(message.id)"
      @toggle="emit('toggle-thought', $event)"
    />

    <ChatToolCallList
      v-if="toolCalls.length"
      :tool-calls="toolCalls"
      :expanded-tool-call-ids="expandedToolCallIds"
      @toggle-tool-call="emit('toggle-tool-call', $event)"
    />

    <ChatContentBlocks v-if="blocks.length" :blocks="blocks" />
    <div v-if="legacyContentWithDOMPurify" class="message-content" v-html="legacyContentWithDOMPurify" />
  </div>
</template>

<style scoped>
.message { margin-bottom: 1rem; padding: 0.75rem; border-radius: 8px; }
.message-user { margin-left: 2rem; background: color-mix(in srgb, var(--border-focus) 24%, var(--bg-panel)); }
.message-assistant { margin-right: 2rem; background: var(--bg-surface); }
.message-header { margin-bottom: 0.5rem; }
.role { color: var(--text-secondary); font-size: var(--text-sm); font-weight: 600; }
.message-content {
  line-height: 1.5;
  overflow-wrap: break-word;
  word-wrap: break-word;
}
.message-content :deep(p) { margin: 0.5rem 0; }
.message-content :deep(ol),
.message-content :deep(ul) { margin: 0.5rem 0; padding-left: 1.5rem; }
.message-content :deep(li) { margin: 0.25rem 0; }
.message-content :deep(pre) {
  padding: 0.75rem;
  border-radius: 4px;
  overflow-x: auto;
  background: var(--bg-code);
  color: var(--text-code);
}
.message-content :deep(code) { font-family: var(--font-mono); font-size: var(--text-base); }
</style>
