<script setup lang="ts">
import { computed } from 'vue';
import { User, Bot, Cog } from 'lucide-vue-next';
import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { Avatar, AvatarFallback } from '@/components/ui/avatar';
import ChatContentBlocks from './ChatContentBlocks.vue';
import ChatThoughtSection from './ChatThoughtSection.vue';
import ChatToolCallList from './ChatToolCallList.vue';
import KnowledgeNoteRenderer from './KnowledgeNoteRenderer.vue';
import MessageActionToolbar from './MessageActionToolbar.vue';
import StopReasonBanner from './StopReasonBanner.vue';
import type { ChatMessage } from '../lib/types';

const props = defineProps<{
  message: ChatMessage;
  expandedThoughtIds: string[];
  expandedToolCallIds: string[];
  isStreaming?: boolean;
  actionsDisabled?: boolean;
  canReconnect?: boolean;
}>();

const emit = defineEmits<{
  'toggle-thought': [messageId: string];
  'toggle-tool-call': [toolCallId: string];
  regenerate: [];
  edit: [{ text: string }];
  reconnect: [];
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
const markdownContent = computed(() =>
  props.message.content && !blocks.value.some((block) => block.type === 'text')
    ? props.message.content
    : '',
);
const streamingTextContent = computed(() => {
  if (!props.isStreaming || !props.message.content) return '';
  if (blocks.value.some((block) => block.type !== 'text')) return '';
  return props.message.content;
});
const isToolOnlyAssistantMessage = computed(
  () =>
    props.message.role === 'assistant' &&
    !props.message.content &&
    !blocks.value.length &&
    toolCalls.value.length > 0,
);
const isUser = computed(() => props.message.role === 'user');
const isAssistant = computed(() => props.message.role === 'assistant');
</script>

<template>
  <div
    class="group mb-4 flex w-full gap-3"
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
      class="min-w-0 max-w-[80%] gap-0 border-border-subtle py-0 shadow-none"
      :class="[
        isUser ? 'bg-surface-high' : 'bg-surface-container',
        isToolOnlyAssistantMessage ? 'px-3 py-1.5' : 'px-3 py-3',
      ]"
    >
      <CardHeader class="flex-row items-center justify-between gap-3 border-b-0 px-0 py-0 pb-2">
        <span class="text-sm font-semibold text-text-secondary">{{ senderLabel }}</span>
        <MessageActionToolbar
          v-if="!actionsDisabled"
          class="opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100"
          :message-role="message.role"
          :message-text="message.content"
          :disabled="Boolean(actionsDisabled)"
          @regenerate="emit('regenerate')"
          @edit="emit('edit', $event)"
        />
      </CardHeader>

      <CardContent class="flex min-w-0 flex-col gap-2 px-0 py-0">
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

        <KnowledgeNoteRenderer
          v-if="streamingTextContent"
          :content="streamingTextContent"
          class="text-text-primary"
        />

        <ChatContentBlocks v-else-if="blocks.length" :blocks="blocks" />

        <KnowledgeNoteRenderer
          v-if="!streamingTextContent && markdownContent"
          :content="markdownContent"
          class="text-text-primary"
        />

        <StopReasonBanner
          v-if="isAssistant"
          :stop-reason="message.stopReason"
          :can-reconnect="canReconnect"
          @reconnect="emit('reconnect')"
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
