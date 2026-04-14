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
.message-content :deep(h1),
.message-content :deep(h2),
.message-content :deep(h3),
.message-content :deep(h4) {
  margin: 0.75rem 0 0.5rem;
  font-weight: var(--weight-title);
  line-height: 1.3;
}
.message-content :deep(h1) { font-size: var(--text-xl); }
.message-content :deep(h2) { font-size: var(--text-lg); }
.message-content :deep(h3) { font-size: var(--text-md); }
.message-content :deep(h4) { font-size: var(--text-base); }
.message-content :deep(p) {
  margin: 0.5rem 0;
  font-size: var(--text-sm);
}
.message-content :deep(ol),
.message-content :deep(ul) { margin: 0.5rem 0; padding-left: 1.5rem; }
.message-content :deep(li) {
  margin: 0.25rem 0;
  font-size: var(--text-sm);
}
.message-content :deep(pre) {
  padding: 0.75rem;
  border-radius: 4px;
  overflow-x: auto;
  background: var(--bg-code);
  color: var(--text-code);
}
.message-content :deep(code) {
  font-family: var(--font-mono);
  font-size: var(--text-sm);
}
.message-content :deep(pre code) { font-size: var(--text-sm); }
.message-content :deep(table) {
  display: block;
  width: 100%;
  margin: 0.75rem 0;
  overflow-x: auto;
  border-collapse: collapse;
  font-size: var(--text-sm);
}
.message-content :deep(th),
.message-content :deep(td) {
  padding: 0.5rem 0.75rem;
  border: 1px solid var(--border-default);
  text-align: left;
}
.message-content :deep(th) {
  background: var(--bg-surface);
  font-size: var(--text-sm);
  font-weight: var(--weight-title);
}
.message-content :deep(td) { font-size: var(--text-sm); }
</style>
