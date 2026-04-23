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
import { Button } from '@/components/ui/button';
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
    <DialogContent class="gap-0 p-0 w-[90%] sm:max-w-[420px]" :show-close-button="false">
      <DialogHeader class="flex flex-row items-start justify-between p-4 px-5">
        <div class="grid gap-2 text-left">
          <DialogTitle>Authentication required</DialogTitle>
          <DialogDescription class="m-0">
            <strong>{{ agentName }}</strong> requires authentication to continue.
            Select an authentication method.
          </DialogDescription>
        </div>
        <DialogClose as-child>
          <Button
            variant="ghost"
            size="icon-sm"
            class="text-[length:var(--text-title)] text-text-secondary hover:text-text-primary p-1"
            aria-label="Close authentication dialog"
          >✕</Button>
        </DialogClose>
      </DialogHeader>

      <div class="p-5">
        <div class="flex flex-col gap-2">
          <Button
            v-for="method in authMethods"
            :key="method.id"
            variant="outline"
            class="group flex items-center justify-between py-3.5 px-4 rounded-sm bg-surface hover:bg-surface-high cursor-pointer text-left transition-all duration-150 ease-out h-auto"
            @click="handleSelect(method.id)"
          >
            <div class="flex flex-col gap-1">
              <span class="font-[var(--weight-display)] text-text-primary">{{ method.name }}</span>
              <span v-if="method.description" class="text-xs text-text-secondary">
                {{ method.description }}
              </span>
            </div>
            <span class="text-[length:var(--text-title)] text-text-secondary group-hover:text-brand">→</span>
          </Button>
        </div>
      </div>

      <DialogFooter class="py-3 px-5">
        <DialogClose as-child>
          <Button variant="outline" class="py-2 px-4 rounded-sm bg-transparent text-text-secondary hover:bg-surface-high text-xs">
            Cancel
          </Button>
        </DialogClose>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
