<script setup lang="ts">
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import type { AppAuthMethod } from '../lib/auth-methods';

defineProps<{
  authMethods: AppAuthMethod[];
  agentName: string;
}>();

const emit = defineEmits<{
  (e: 'select', methodId: string): void;
  (e: 'cancel'): void;
}>();

function handleSelect(methodId: string) {
  emit('select', methodId);
}
</script>

<template>
  <Dialog :open="true" @update:open="(open) => !open && emit('cancel')">
    <DialogContent class="auth-dialog gap-0 p-0 sm:max-w-[420px]" :show-close-button="false">
      <DialogHeader class="dialog-header">
        <div class="dialog-heading">
          <DialogTitle>Authentication required</DialogTitle>
          <DialogDescription class="description">
            <strong>{{ agentName }}</strong> requires authentication to continue.
            Select an authentication method.
          </DialogDescription>
        </div>
        <DialogClose as-child>
          <button type="button" class="close-btn" aria-label="Close authentication dialog">✕</button>
        </DialogClose>
      </DialogHeader>

      <div class="dialog-content">
        <div class="auth-methods">
          <button
            v-for="method in authMethods"
            :key="method.id"
            type="button"
            class="auth-method-btn"
            @click="handleSelect(method.id)"
          >
            <div class="method-info">
              <span class="method-name">{{ method.name }}</span>
              <span v-if="method.description" class="method-desc">
                {{ method.description }}
              </span>
            </div>
            <span class="arrow">→</span>
          </button>
        </div>
      </div>

      <DialogFooter class="dialog-footer">
        <DialogClose as-child>
          <button type="button" class="cancel-btn">
            Cancel
          </button>
        </DialogClose>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>

<style scoped>
.auth-dialog {
  width: 90%;
  max-width: 420px;
}

.dialog-header {
  display: flex;
  flex-direction: row;
  align-items: flex-start;
  justify-content: space-between;
  padding: 1rem 1.25rem;
}

.dialog-heading {
  display: grid;
  gap: 0.5rem;
  text-align: left;
}

.close-btn {
  border: none;
  background: transparent;
  font-size: var(--text-title);
  cursor: pointer;
  color: var(--text-dim);
  padding: 0.25rem;
}

.close-btn:hover {
  color: var(--color-text-primary);
}

.dialog-content {
  padding: 1.25rem;
}

.description {
  margin: 0;
}

.auth-methods {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.auth-method-btn {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.875rem 1rem;
  border: 1px solid rgba(66, 71, 83, 0.15);
  border-radius: 4px;
  background: var(--surface);
  cursor: pointer;
  text-align: left;
  transition: all 0.15s ease;
}

.auth-method-btn:hover {
  background: var(--surface-high);
}

.method-info {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.method-name {
  font-weight: var(--weight-display);
  color: var(--color-text-primary);
}

.method-desc {
  font-size: 0.75rem;
  color: var(--text-dim);
}

.arrow {
  font-size: var(--text-title);
  color: var(--text-dim);
}

.auth-method-btn:hover .arrow {
  color: var(--color-brand);
}

.dialog-footer {
  padding: 0.75rem 1.25rem;
}

.cancel-btn {
  padding: 0.5rem 1rem;
  border: 1px solid rgba(66, 71, 83, 0.15);
  border-radius: 4px;
  background: transparent;
  color: var(--text-dim);
  cursor: pointer;
  font-size: 0.75rem;
}

.cancel-btn:hover {
  background: var(--surface-high);
}
</style>
