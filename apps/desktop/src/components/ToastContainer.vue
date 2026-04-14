<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { useToastStore, type ToastLevel } from '../stores/toast';

const toastStore = useToastStore();
const { toasts } = storeToRefs(toastStore);

const ACCENT_BY_LEVEL: Record<ToastLevel, string> = {
  info: '#4ea1ff',
  warning: '#d9a441',
  error: '#e05a5a',
  success: '#4fb36c',
};
</script>

<template>
  <Teleport to="body">
    <TransitionGroup
      v-if="toasts.length"
      name="toast"
      tag="section"
      class="toast-container"
      aria-live="polite"
      aria-label="Notifications"
    >
      <article
        v-for="toast in toasts"
        :key="toast.id"
        class="toast-card"
        :style="{ '--toast-accent': ACCENT_BY_LEVEL[toast.level] }"
        role="status"
      >
        <p class="toast-message">{{ toast.message }}</p>
        <button
          v-if="toast.dismissable"
          type="button"
          class="toast-dismiss"
          aria-label="Dismiss notification"
          @click="toastStore.removeToast(toast.id)"
        >
          ×
        </button>
      </article>
    </TransitionGroup>
  </Teleport>
</template>

<style scoped>
.toast-container {
  position: fixed;
  right: 1rem;
  bottom: 1rem;
  z-index: 9999;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  width: min(24rem, calc(100vw - 2rem));
  max-height: min(15rem, calc(100vh - 2rem));
  overflow-y: auto;
  pointer-events: none;
}

.toast-card {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: start;
  gap: 0.75rem;
  padding: 0.8rem 0.9rem;
  border-left: 3px solid var(--toast-accent);
  border-radius: 0.75rem;
  background: #1e1e1e;
  color: #f5f5f5;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
  pointer-events: auto;
}

.toast-message {
  margin: 0;
  font-size: var(--text-sm);
  line-height: 1.45;
  word-break: break-word;
}

.toast-dismiss {
  border: none;
  background: transparent;
  color: inherit;
  cursor: pointer;
  font-size: var(--text-lg);
  line-height: 1;
  padding: 0;
}

.toast-dismiss:hover {
  opacity: 0.75;
}

.toast-enter-active,
.toast-leave-active,
.toast-move {
  transition: opacity 180ms ease, transform 180ms ease;
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateY(8px);
}
</style>
