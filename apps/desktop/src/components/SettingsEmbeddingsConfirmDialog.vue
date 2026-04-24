<script setup lang="ts">
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';

defineProps<{
  open: boolean;
  type: 'activate' | 'delete' | 'regenerate' | null;
  modelName?: string | null;
}>();

const emit = defineEmits<{
  'update:open': [value: boolean];
  confirm: [];
}>();
</script>

<template>
  <AlertDialog :open="open" @update:open="(v: boolean) => emit('update:open', v)">
    <AlertDialogContent class="sm:max-w-[28rem]">
      <AlertDialogHeader>
        <AlertDialogTitle>
          <template v-if="type === 'activate'">Activate {{ modelName ?? 'model' }}?</template>
          <template v-else-if="type === 'regenerate'">Regenerate all embeddings?</template>
          <template v-else>Delete {{ modelName ?? 'model' }}?</template>
        </AlertDialogTitle>
        <AlertDialogDescription>
          <template v-if="type === 'activate'">
            This will rebuild all vector embeddings. Search will be temporarily unavailable while the index is regenerated.
          </template>
          <template v-else-if="type === 'regenerate'">
            This will delete all vector embeddings and rebuild them from scratch. Search will be unavailable until the rebuild completes.
          </template>
          <template v-else>
            This will remove the downloaded model files from disk. You can download the model again later if you change your mind.
          </template>
        </AlertDialogDescription>
      </AlertDialogHeader>
      <AlertDialogFooter>
        <AlertDialogCancel>Cancel</AlertDialogCancel>
        <AlertDialogAction @click="emit('confirm')">
          <template v-if="type === 'activate'">Activate</template>
          <template v-else-if="type === 'regenerate'">Regenerate</template>
          <template v-else>Delete</template>
        </AlertDialogAction>
      </AlertDialogFooter>
    </AlertDialogContent>
  </AlertDialog>
</template>
