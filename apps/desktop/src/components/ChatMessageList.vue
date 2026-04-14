<script setup lang="ts">
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
}>();
</script>

<template>
  <div role="log" aria-live="polite" aria-label="Chat messages">
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
.permission-status {
  margin-top: 1rem;
  padding: 0.75rem 1rem;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-warning);
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
  border-top-color: var(--ring);
  border-radius: 50%;
  animation: spin 1s linear infinite;
}
.cancel-btn {
  margin-left: auto;
  padding: 0.25rem 0.5rem;
  border: 1px solid var(--border);
  border-radius: 4px;
  background: transparent;
  font-size: var(--text-sm);
  cursor: pointer;
}
@keyframes spin { to { transform: rotate(360deg); } }
</style>
