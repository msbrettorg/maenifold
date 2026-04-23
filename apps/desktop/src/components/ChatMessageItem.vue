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
const senderLabel = computed(() => {
  switch (props.message.role) {
    case 'user':
      return 'You';
    case 'assistant':
      return 'Assistant';
    default:
      return 'System';
  }
});
const formattedTimestamp = computed(() => {
  const date = new Date(props.message.timestamp);
  return Number.isNaN(date.getTime()) ? 'unknown time' : date.toLocaleTimeString();
});
const messageAriaLabel = computed(() => `${senderLabel.value} message at ${formattedTimestamp.value}`);
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
  const html = DOMPurify.sanitize(typeof parsed === 'string' ? parsed : '');
  // Guard: if markdown consumed the input into empty HTML structures
  // (e.g. "\n\n4." → <ol><li></li></ol>), fall back to escaped plaintext.
  const visibleText = html.replace(/<[^>]*>/g, '').trim();
  if (visibleText.length === 0 && content.trim().length > 0) {
    const escaped = content
      .trim()
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;');
    return `<p>${escaped}</p>`;
  }
  return html;
}
</script>

<template>
  <div
    class="mb-4 p-3 rounded"
    :class="[
      message.role === 'user' ? 'ml-auto max-w-[80%] bg-[var(--surface-high)]' : '',
      message.role === 'assistant' ? 'bg-[var(--surface-container)]' : '',
      isToolOnlyAssistantMessage ? 'pt-1.5 pb-1.5' : '',
    ]"
    role="article"
    :aria-label="messageAriaLabel"
  >
    <div class="mb-2">
      <span class="text-text-secondary text-sm font-[number:var(--weight-title)]">{{ senderLabel }}</span>
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
    <div v-if="legacyContentWithDOMPurify" class="message-content leading-[1.5] [overflow-wrap:break-word] [word-wrap:break-word]" v-html="legacyContentWithDOMPurify" />
  </div>
</template>

<!-- audit-ok: max-style-rules — deep-third-party: :deep() rules style sanitized markdown HTML rendered via v-html (h1–h4, p, ul/ol, pre, code, table, th/td) inside .message-content; Tailwind utilities cannot reach this child markup across the scoped boundary. -->
<style scoped>
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
  background: var(--color-bg-app);
  color: var(--color-text-secondary);
  font-family: var(--font-mono);
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
  border: 1px solid var(--color-border-subtle);
  text-align: left;
}
.message-content :deep(th) {
  background: var(--surface-high);
  font-size: var(--text-sm);
  font-weight: var(--weight-title);
}
.message-content :deep(td) { font-size: var(--text-sm); }
</style>
