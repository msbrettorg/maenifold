<script setup lang="ts">
import { Button } from '@/components/ui/button';
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
    <DialogContent class="sm:max-w-lg w-[min(28rem,100%)] grid gap-4" :show-close-button="false">
      <DialogHeader class="flex flex-row items-start gap-3">
        <span class="text-[length:var(--text-title)] leading-none" aria-hidden="true">🔑</span>
        <div class="grid gap-1 text-left">
          <DialogTitle>{{ action.label }}</DialogTitle>
          <DialogDescription>{{ action.description }}</DialogDescription>
        </div>
      </DialogHeader>

      <p class="m-0 text-[var(--text-secondary)] text-[0.75rem]">
        {{ server }} needs setup before this tool can be used from the palette.
      </p>
      <p v-if="error" class="m-0 text-[var(--color-status-abandoned)] text-[0.75rem]" role="alert">{{ error }}</p>

      <DialogFooter class="flex flex-row gap-3 justify-between">
        <DialogClose as-child>
          <Button variant="ghost" :disabled="busy">Cancel</Button>
        </DialogClose>
        <Button data-testid="tool-auth-setup-confirm" :disabled="busy" @click="emit('confirm')">
          {{ busy ? 'Setting up…' : 'Set up' }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
