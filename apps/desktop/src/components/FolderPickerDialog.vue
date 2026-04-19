<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { isAbsolutePath } from '../lib/acp-meta';
import { Button } from './ui/button';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from './ui/dialog';
import { Input } from './ui/input';

const props = defineProps<{
  open: boolean;
  initialPath: string;
}>();

const emit = defineEmits<{
  (e: 'confirm', path: string): void;
  (e: 'cancel'): void;
}>();

const path = ref('');

watch(
  () => [props.open, props.initialPath] as const,
  ([open, initialPath]) => {
    if (open) {
      path.value = initialPath;
    }
  },
  { immediate: true },
);

const trimmedPath = computed(() => path.value.trim());
const isValidAbsolutePath = computed(() => isAbsolutePath(trimmedPath.value));
const showValidationError = computed(() =>
  trimmedPath.value.length > 0 && !isValidAbsolutePath.value,
);

function handleConfirm() {
  if (!isValidAbsolutePath.value) return;
  emit('confirm', trimmedPath.value);
}
</script>

<template>
  <Dialog :open="open" @update:open="(nextOpen) => !nextOpen && emit('cancel')">
    <DialogContent class="gap-4 p-0 sm:max-w-[32rem]" :show-close-button="false">
      <DialogHeader class="px-6 pt-6">
        <DialogTitle>Enter Working Directory</DialogTitle>
      </DialogHeader>

      <div class="px-6">
        <Input
          v-model="path"
          type="text"
          placeholder="/absolute/path"
          autofocus
        />
        <p v-if="showValidationError" class="mt-2 text-xs text-danger" role="alert">
          Working directory must be an absolute path.
        </p>
      </div>

      <DialogFooter class="gap-2 px-6 pb-6">
        <Button type="button" variant="ghost" @click="emit('cancel')">Cancel</Button>
        <Button
          type="button"
          :disabled="trimmedPath.length === 0 || !isValidAbsolutePath"
          @click="handleConfirm"
        >
          Use Path
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
