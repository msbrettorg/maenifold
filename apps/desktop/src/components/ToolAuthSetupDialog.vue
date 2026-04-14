<script setup lang="ts">
import type { AuthSetupAction } from '../lib/palette-health';

defineProps<{
  action: AuthSetupAction;
  server: string;
  busy?: boolean;
  error?: string | null;
}>();

const emit = defineEmits<{
  close: [];
  confirm: [];
}>();
</script>

<template>
  <div class="auth-setup-overlay" @click.self="emit('close')">
    <div class="auth-setup-dialog" role="dialog" aria-modal="true" aria-labelledby="auth-setup-title">
      <div class="dialog-header">
        <span class="dialog-icon">🔑</span>
        <div>
          <h3 id="auth-setup-title">{{ action.label }}</h3>
          <p>{{ action.description }}</p>
        </div>
      </div>

      <p class="dialog-body">
        {{ server }} needs setup before this tool can be used from the palette.
      </p>
      <p v-if="error" class="dialog-error" role="alert">{{ error }}</p>

      <div class="dialog-actions">
        <button type="button" class="btn btn-ghost" :disabled="busy" @click="emit('close')">Cancel</button>
        <button type="button" class="btn btn-primary" :disabled="busy" @click="emit('confirm')">
          {{ busy ? 'Setting up…' : 'Set up' }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.auth-setup-overlay {
  position: fixed;
  inset: 0;
  z-index: 1100;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1.5rem;
  background: var(--bg-overlay);
}

.auth-setup-dialog {
  width: min(28rem, 100%);
  display: grid;
  gap: 1rem;
  padding: 1rem;
  border: 1px solid var(--border-default);
  border-radius: 0.9rem;
  background: var(--bg-main);
}

.dialog-header,
.dialog-actions {
  display: flex;
  gap: 0.75rem;
}

.dialog-header {
  align-items: flex-start;
}

.dialog-actions {
  justify-content: flex-end;
}

.dialog-header h3,
.dialog-header p,
.dialog-body,
.dialog-error {
  margin: 0;
}

.dialog-header p,
.dialog-body {
  color: var(--text-secondary);
}

.dialog-icon {
  font-size: var(--text-title);
  line-height: 1;
}

.dialog-error {
  color: var(--color-status-abandoned);
  font-size: var(--text-caption);
}
</style>
