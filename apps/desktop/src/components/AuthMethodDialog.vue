<script setup lang="ts">
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
  <div class="auth-overlay" @click.self="emit('cancel')">
    <div class="auth-dialog">
      <div class="dialog-header">
        <h3>Authentication Required</h3>
        <button class="close-btn" @click="emit('cancel')">✕</button>
      </div>
      
      <div class="dialog-content">
        <p class="description">
          <strong>{{ agentName }}</strong> requires authentication to continue.
          Select an authentication method:
        </p>
        
        <div class="auth-methods">
          <button
            v-for="method in authMethods"
            :key="method.id"
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
      
      <div class="dialog-footer">
        <button class="cancel-btn" @click="emit('cancel')">
          Cancel
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.auth-overlay {
  position: fixed;
  inset: 0;
  background: var(--bg-overlay);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

@supports ((backdrop-filter: blur(1px)) or (-webkit-backdrop-filter: blur(1px))) {
  .auth-overlay {
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    background: rgba(0, 0, 0, 0.4);
  }
}

.auth-dialog {
  background: var(--bg-main);
  border-radius: 8px;
  width: 90%;
  max-width: 420px;
  box-shadow: 0 4px 20px color-mix(in srgb, var(--bg-main) 30%, transparent);
}

.dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1rem 1.25rem;
  border-bottom: 1px solid var(--border-default);
}

.dialog-header h3 {
  margin: 0;
  font-size: var(--text-xl);
}

.close-btn {
  border: none;
  background: transparent;
  font-size: var(--text-xl);
  cursor: pointer;
  color: var(--text-secondary);
  padding: 0.25rem;
}

.close-btn:hover {
  color: var(--text-primary);
}

.dialog-content {
  padding: 1.25rem;
}

.description {
  margin: 0 0 1rem 0;
  color: var(--text-secondary);
  font-size: var(--text-base);
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
  border: 1px solid var(--border-default);
  border-radius: 6px;
  background: var(--bg-panel);
  cursor: pointer;
  text-align: left;
  transition: all 0.15s ease;
}

.auth-method-btn:hover {
  border-color: var(--border-focus);
  background: var(--bg-surface);
}

.method-info {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.method-name {
  font-weight: 500;
  color: var(--text-primary);
}

.method-desc {
  font-size: var(--text-sm);
  color: var(--text-muted);
}

.arrow {
  font-size: var(--text-xl);
  color: var(--text-muted);
}

.auth-method-btn:hover .arrow {
  color: var(--border-focus);
}

.dialog-footer {
  padding: 0.75rem 1.25rem;
  border-top: 1px solid var(--border-default);
  display: flex;
  justify-content: flex-end;
}

.cancel-btn {
  padding: 0.5rem 1rem;
  border: 1px solid var(--border-default);
  border-radius: 4px;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  font-size: var(--text-base);
}

.cancel-btn:hover {
  background: var(--bg-surface);
}
</style>
