<script setup lang="ts">
import { Search, FileText, ClipboardList } from 'lucide-vue-next';
import type { Component } from 'vue';
import { Button } from '@/components/ui/button';
import PlanView from './PlanView.vue';
import ChatMessageItem from './ChatMessageItem.vue';
import type { ChatMessage, PlanEntry } from '../lib/types';

defineProps<{
  messages: ChatMessage[];
  currentPlan: PlanEntry[] | null;
  expandedThoughtIds: string[];
  expandedToolCallIds: string[];
  pendingPermissionCount: number;
  isLoading: boolean;
}>();

const emit = defineEmits<{
  cancel: [];
  'toggle-thought': [messageId: string];
  'toggle-tool-call': [toolCallId: string];
  prefill: [text: string];
  regenerate: [messageId: string];
  edit: [{ messageId: string; text: string }];
}>();

const starterChips: { icon: Component; label: string }[] = [
  { icon: Search, label: 'Research a topic' },
  { icon: FileText, label: 'Review code' },
  { icon: ClipboardList, label: 'Plan a sprint' },
];
</script>

<template>
  <div role="log" aria-live="polite" aria-label="Chat messages">
    <!-- Empty state with starter chips -->
    <div v-if="messages.length === 0 && !isLoading" class="flex flex-col items-center gap-3 px-4 py-12 text-center">
      <p class="m-0 text-text-secondary text-md">Start a conversation or try one of these:</p>
      <div class="flex flex-wrap justify-center gap-2">
        <Button
          v-for="chip in starterChips"
          :key="chip.label"
          variant="outline"
          class="inline-flex items-center gap-1.5 px-3.5 py-2 border border-border-subtle rounded bg-surface-high text-text-primary font-[inherit] text-sm cursor-pointer transition-colors duration-150 hover:bg-surface-highest"
          @click="emit('prefill', chip.label)"
        >
          <span class="text-sm" aria-hidden="true"><component :is="chip.icon" class="size-4" :stroke-width="1.5" /></span>
          <span>{{ chip.label }}</span>
        </Button>
      </div>
    </div>

    <PlanView v-if="currentPlan" :entries="currentPlan" />

    <ChatMessageItem
      v-for="message in messages"
      :key="message.id"
      :message="message"
      :is-streaming="isLoading && message.role === 'assistant' && message === messages[messages.length - 1]"
      :actions-disabled="isLoading"
      :expanded-thought-ids="expandedThoughtIds"
      :expanded-tool-call-ids="expandedToolCallIds"
      @toggle-thought="emit('toggle-thought', $event)"
      @toggle-tool-call="emit('toggle-tool-call', $event)"
      @regenerate="emit('regenerate', message.id)"
      @edit="emit('edit', { messageId: message.id, text: $event.text })"
    />

    <div v-if="pendingPermissionCount > 0" class="mt-4 px-4 py-3 rounded bg-bg-warning border border-border-subtle text-text-primary">
      {{ pendingPermissionCount }} permission request{{ pendingPermissionCount === 1 ? '' : 's' }} awaiting review.
    </div>

    <div v-if="isLoading" class="flex items-center gap-2 p-3 text-text-secondary">
      <span class="inline-block w-4 h-4 rounded-full border-2 border-border-subtle border-t-brand animate-spin"></span>
      <span>Thinking...</span>
      <Button variant="outline" size="sm" class="ml-auto px-2 py-1 border border-border-subtle rounded bg-transparent text-text-primary text-sm cursor-pointer" @click="emit('cancel')">Cancel</Button>
    </div>
  </div>
</template>
