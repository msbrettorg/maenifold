<script setup lang="ts">
import { ref } from 'vue';
import { Square } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { useWorkflowStore } from '../stores/workflow';

const props = defineProps<{
  sessionId: string;
  disabled?: boolean;
}>();

const workflowStore = useWorkflowStore();
const stopping = ref(false);
const error = ref<string | null>(null);

async function stop() {
  stopping.value = true;
  error.value = null;
  try {
    await workflowStore.cancelWorkflow(props.sessionId);
  } catch (unknownError) {
    error.value = unknownError instanceof Error ? unknownError.message : String(unknownError);
  } finally {
    stopping.value = false;
  }
}
</script>

<template>
  <div class="flex flex-col items-end gap-1">
    <Button
      type="button"
      variant="outline"
      class="gap-1.5 border-destructive/40 text-destructive hover:bg-destructive/10"
      :disabled="disabled"
      :loading="stopping"
      data-testid="workflow-stop-button"
      @click="stop"
    >
      <Square class="size-4" aria-hidden="true" />
      Stop
    </Button>
    <p v-if="error" role="alert" class="m-0 max-w-80 text-right text-xs text-destructive">
      {{ error }}
    </p>
  </div>
</template>
