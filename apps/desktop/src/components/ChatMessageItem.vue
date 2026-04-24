<script setup lang="ts">
import { computed } from 'vue';
import DOMPurify from 'dompurify';
import { marked } from 'marked';
import { User, Bot, Cog } from 'lucide-vue-next';
import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { Avatar, AvatarFallback } from '@/components/ui/avatar';
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
const messageAriaLabel = computed(
  () => `${senderLabel.value} message at ${formattedTimestamp.value}`,
);
const legacyContentWithDOMPurify = computed(() =>
  props.message.content && !blocks.value.some((block) => block.type === 'text')
    ? renderMarkdown(props.message.content)
    : null,
);
const isToolOnlyAssistantMessage = computed(
  () =>
    props.message.role === 'assistant' &&
    !props.message.content &&
    !blocks.value.length &&
    toolCalls.value.length > 0,
);
const isUser = computed(() => props.message.role === 'user');
const isAssistant = computed(() => props.message.role === 'assistant');

function renderMarkdown(content: string): string {
  const parsed = marked.parse(content, { async: false });
  const html = DOMPurify.sanitize(typeof parsed === 'string' ? parsed : '');
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
    class="mb-4 flex w-full gap-3"
    :class="isUser ? 'justify-end' : 'justify-start'"
    role="article"
    :aria-label="messageAriaLabel"
  >
    <Avatar v-if="!isUser" class="mt-1 shrink-0">
      <AvatarFallback class="bg-surface-high text-text-primary">
        <Bot v-if="isAssistant" class="size-4" aria-hidden="true" />
        <Cog v-else class="size-4" aria-hidden="true" />
      </AvatarFallback>
    </Avatar>

    <Card
      class="max-w-[80%] gap-0 border-border-subtle py-0 shadow-none"
      :class="[
        isUser ? 'bg-surface-high' : 'bg-surface-container',
        isToolOnlyAssistantMessage ? 'px-3 py-1.5' : 'px-3 py-3',
      ]"
    >
      <CardHeader class="gap-0 border-b-0 px-0 py-0 pb-2">
        <span class="text-sm font-semibold text-text-secondary">{{ senderLabel }}</span>
      </CardHeader>

      <CardContent class="flex flex-col gap-2 px-0 py-0">
        <ChatThoughtSection
          v-if="message.thought && isAssistant"
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

        <div
          v-if="legacyContentWithDOMPurify"
          class="text-sm leading-relaxed break-words text-text-primary [&_code]:font-mono [&_code]:text-sm [&_h1]:mt-3 [&_h1]:mb-2 [&_h1]:text-xl [&_h1]:font-semibold [&_h1]:leading-tight [&_h2]:mt-3 [&_h2]:mb-2 [&_h2]:text-lg [&_h2]:font-semibold [&_h2]:leading-tight [&_h3]:mt-3 [&_h3]:mb-2 [&_h3]:text-base [&_h3]:font-semibold [&_h3]:leading-tight [&_h4]:mt-3 [&_h4]:mb-2 [&_h4]:text-sm [&_h4]:font-semibold [&_h4]:leading-tight [&_li]:my-1 [&_li]:text-sm [&_ol]:my-2 [&_ol]:pl-6 [&_p]:my-2 [&_pre]:my-2 [&_pre]:overflow-x-auto [&_pre]:rounded [&_pre]:bg-surface [&_pre]:p-3 [&_pre]:font-mono [&_pre]:text-text-secondary [&_pre_code]:text-sm [&_table]:my-3 [&_table]:block [&_table]:w-full [&_table]:overflow-x-auto [&_table]:border-collapse [&_table]:text-sm [&_td]:border [&_td]:border-border-subtle [&_td]:px-3 [&_td]:py-2 [&_td]:text-left [&_td]:text-sm [&_th]:border [&_th]:border-border-subtle [&_th]:bg-surface-high [&_th]:px-3 [&_th]:py-2 [&_th]:text-left [&_th]:text-sm [&_th]:font-semibold [&_ul]:my-2 [&_ul]:pl-6"
          v-html="legacyContentWithDOMPurify"
        />
      </CardContent>
    </Card>

    <Avatar v-if="isUser" class="mt-1 shrink-0">
      <AvatarFallback class="bg-surface-high text-text-primary">
        <User class="size-4" aria-hidden="true" />
      </AvatarFallback>
    </Avatar>
  </div>
</template>
