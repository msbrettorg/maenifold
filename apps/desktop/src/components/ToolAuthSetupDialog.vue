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
  <Dialog :open="true" @update:open="(open) => !open && emit('close')">
    <DialogContent class="auth-setup-dialog sm:max-w-lg" :show-close-button="false">
      <DialogHeader class="dialog-header">
        <span class="dialog-icon" aria-hidden="true">🔑</span>
        <div class="dialog-copy">
          <DialogTitle>{{ action.label }}</DialogTitle>
          <DialogDescription>{{ action.description }}</DialogDescription>
        </div>
      </DialogHeader>

      <p class="dialog-body">
        {{ server }} needs setup before this tool can be used from the palette.
      </p>
      <p v-if="error" class="dialog-error" role="alert">{{ error }}</p>

      <DialogFooter class="dialog-actions">
        <DialogClose as-child>
          <button type="button" class="btn btn-ghost" :disabled="busy">Cancel</button>
        </DialogClose>
        <button type="button" class="btn btn-primary" :disabled="busy" @click="emit('confirm')">
          {{ busy ? 'Setting up…' : 'Set up' }}
        </button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>

<style scoped>
.auth-setup-dialog {
  width: min(28rem, 100%);
  display: grid;
  gap: 1rem;
}

.dialog-header,
.dialog-actions {
  display: flex;
  gap: 0.75rem;
}

.dialog-header {
  flex-direction: row;
  align-items: flex-start;
}

.dialog-copy {
  display: grid;
  gap: 0.25rem;
  text-align: left;
}

.dialog-body,
.dialog-error {
  margin: 0;
}

.dialog-body {
  color: var(--muted-foreground);
}

.dialog-icon {
  font-size: var(--text-title);
  line-height: 1;
}

.dialog-error {
  color: var(--color-status-abandoned);
  font-size: var(--text-caption);
}

.dialog-actions {
  flex-direction: row;
  justify-content: flex-end;
}
</style>
