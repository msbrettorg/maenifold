<script setup lang="ts">
import { computed, ref } from 'vue';
import { Play } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { useWorkflowStore } from '../stores/workflow';
import type { WorkflowAssetDefinition } from '../lib/workflow-asset-types';
import { normalizeWorkflowInputs } from '../lib/workflow-inputs';
import WorkflowParameterDialog from './WorkflowParameterDialog.vue';

const props = defineProps<{
  workflow: WorkflowAssetDefinition;
}>();

const emit = defineEmits<{
  started: [sessionId: string];
}>();

const workflowStore = useWorkflowStore();
const dialogOpen = ref(false);
const inlineError = ref<string | null>(null);

const inputFields = computed(() => normalizeWorkflowInputs(props.workflow.inputs));
const hasInputs = computed(() => inputFields.value.length > 0);

async function dispatch(values: Record<string, unknown> = {}) {
  inlineError.value = null;
  try {
    const sessionId = await workflowStore.runWorkflow(props.workflow.id, values);
    dialogOpen.value = false;
    emit('started', sessionId);
  } catch (error) {
    inlineError.value = error instanceof Error ? error.message : String(error);
  }
}

function handleRun() {
  if (hasInputs.value) {
    dialogOpen.value = true;
    return;
  }
  void dispatch();
}
</script>

<template>
  <div class="flex flex-col items-end gap-1">
    <Button
      type="button"
      class="gap-1.5"
      :loading="workflowStore.runningWorkflow"
      data-testid="workflow-run-button"
      @click="handleRun"
    >
      <Play class="size-4" aria-hidden="true" />
      Run
    </Button>
    <p v-if="inlineError" role="alert" class="m-0 max-w-80 text-right text-xs text-destructive">
      {{ inlineError }}
    </p>
    <WorkflowParameterDialog
      v-model:open="dialogOpen"
      :inputs="workflow.inputs"
      :error="inlineError"
      :submitting="workflowStore.runningWorkflow"
      @submit="dispatch"
    />
  </div>
</template>
