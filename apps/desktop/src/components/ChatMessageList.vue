<script setup lang="ts">
import { Search, FileText, ClipboardList } from 'lucide-vue-next';
import type { Component } from 'vue';
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
    <div v-if="messages.length === 0 && !isLoading" class="chat-empty">
      <p class="chat-empty-hint">Start a conversation or try one of these:</p>
      <div class="chat-empty-chips">
        <button
          v-for="chip in starterChips"
          :key="chip.label"
          type="button"
          class="starter-chip"
          @click="emit('prefill', chip.label)"
        >
          <span class="chip-icon" aria-hidden="true"><component :is="chip.icon" class="size-4" :stroke-width="1.5" /></span>
          <span>{{ chip.label }}</span>
        </button>
      </div>
    </div>

    <PlanView v-if="currentPlan" :entries="currentPlan" />

    <ChatMessageItem
      v-for="message in messages"
      :key="message.id"
      :message="message"
      :expanded-thought-ids="expandedThoughtIds"
      :expanded-tool-call-ids="expandedToolCallIds"
      @toggle-thought="emit('toggle-thought', $event)"
      @toggle-tool-call="emit('toggle-tool-call', $event)"
    />

    <div v-if="pendingPermissionCount > 0" class="permission-status">
      {{ pendingPermissionCount }} permission request{{ pendingPermissionCount === 1 ? '' : 's' }} awaiting review.
    </div>

    <div v-if="isLoading" class="loading-indicator">
      <span class="spinner"></span>
      <span>Thinking...</span>
      <button type="button" class="cancel-btn" @click="emit('cancel')">Cancel</button>
    </div>
  </div>
</template>

<style scoped>
.chat-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.75rem;
  padding: 3rem 1rem;
  text-align: center;
}
.chat-empty-hint {
  margin: 0;
  color: var(--muted-foreground);
  font-size: var(--text-md);
}
.chat-empty-chips {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 0.5rem;
}
.starter-chip {
  display: inline-flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.5rem 0.875rem;
  border: 1px solid rgba(66, 71, 83, 0.15);
  border-radius: 4px;
  background: var(--surface-high);
  color: var(--foreground);
  font: inherit;
  font-size: var(--text-sm);
  cursor: pointer;
  transition: background 0.15s;
}
.starter-chip:hover { background: var(--surface-highest); }
.chip-icon { font-size: var(--text-sm); }
.permission-status {
  margin-top: 1rem;
  padding: 0.75rem 1rem;
  border-radius: 4px;
  background: var(--bg-warning);
  border: 1px solid rgba(66, 71, 83, 0.15);
  color: var(--foreground);
}
.loading-indicator {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem;
  color: var(--muted-foreground);
}
.spinner {
  width: 16px;
  height: 16px;
  border: 2px solid var(--border);
  border-top-color: var(--blue);
  border-radius: var(--radius);
  animation: spin 1s linear infinite;
}
.cancel-btn {
  margin-left: auto;
  padding: 0.25rem 0.5rem;
  border: 1px solid rgba(66, 71, 83, 0.15);
  border-radius: 4px;
  background: transparent;
  color: var(--foreground);
  font-size: var(--text-sm);
  cursor: pointer;
}
@keyframes spin { to { transform: rotate(360deg); } }
</style>
