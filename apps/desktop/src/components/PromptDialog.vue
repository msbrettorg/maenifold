<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { Button } from './ui/button';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from './ui/dialog';
import { Input } from './ui/input';
import { usePromptDialogStore } from '../stores/prompt-dialog';

const promptDialogStore = usePromptDialogStore();
const {
  isOpen,
  title,
  description,
  placeholder,
  confirmLabel,
  initialValue,
} = storeToRefs(promptDialogStore);

const value = ref('');
const trimmedValue = computed(() => value.value.trim());

watch(
  () => [isOpen.value, initialValue.value] as const,
  ([open, nextInitialValue]) => {
    if (open) {
      value.value = nextInitialValue;
    }
  },
  { immediate: true },
);

function handleConfirm(): void {
  promptDialogStore.confirm(trimmedValue.value);
}
</script>

<template>
  <Dialog :open="isOpen" @update:open="(open) => !open && promptDialogStore.cancel()">
    <DialogContent class="gap-4 p-0 sm:max-w-[28rem]" :show-close-button="false">
      <DialogHeader class="px-6 pt-6">
        <DialogTitle>{{ title }}</DialogTitle>
        <p v-if="description" class="m-0 text-xs text-text-secondary">{{ description }}</p>
      </DialogHeader>

      <div class="px-6">
        <Input v-model="value" type="text" :placeholder="placeholder" autofocus />
      </div>

      <DialogFooter class="gap-2 px-6 pb-6">
        <Button type="button" variant="ghost" @click="promptDialogStore.cancel()">Cancel</Button>
        <Button type="button" @click="handleConfirm()">{{ confirmLabel }}</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
