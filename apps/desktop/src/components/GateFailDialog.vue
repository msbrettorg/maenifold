<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { useGateMcp } from '../composables/useGateMcp';
import type { GateTask } from '../lib/gate';
import { nextFailStage, stageAgent } from '../lib/gate-pipelines';

const props = defineProps<{ open: boolean; root: string; task: GateTask }>();
const emit = defineEmits<{ 'update:open': [value: boolean]; completed: [] }>();

const gateMcp = useGateMcp();
const notes = ref('');
const error = ref('');
const submitting = ref(false);
const canSubmit = computed(() => notes.value.trim().length > 0 && !submitting.value);
const failStage = computed(() => nextFailStage(props.task));

watch(() => props.open, (open) => {
  if (open) {
    notes.value = '';
    error.value = '';
  }
});

async function submit() {
  if (!notes.value.trim()) {
    error.value = 'Notes are required.';
    return;
  }
  submitting.value = true;
  error.value = '';
  try {
    await gateMcp.fail({
      root: props.root,
      taskId: props.task.taskId,
      agent: stageAgent(props.task),
      notes: notes.value.trim(),
    });
    emit('completed');
    emit('update:open', false);
  } catch (errorValue) {
    error.value = errorValue instanceof Error ? errorValue.message : String(errorValue);
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <Dialog :open="open" @update:open="emit('update:open', $event)">
    <DialogContent v-if="open" class="border-border-card bg-bg-surface text-text-primary sm:max-w-lg">
      <DialogHeader>
        <DialogTitle>Fail stage</DialogTitle>
        <DialogDescription>
          Failing `{{ task.state }}` routes this task to `{{ failStage }}`.
        </DialogDescription>
      </DialogHeader>
      <form class="grid gap-4" data-testid="gate-fail-form" @submit.prevent="submit">
        <Alert v-if="error" variant="destructive">
          <AlertDescription>{{ error }}</AlertDescription>
        </Alert>
        <div class="grid gap-2">
          <Label for="gate-fail-notes">Notes</Label>
          <Textarea id="gate-fail-notes" v-model="notes" data-testid="gate-fail-notes" class="min-h-32 border-border-subtle bg-bg-app" />
        </div>
        <DialogFooter>
          <Button type="button" variant="ghost" data-testid="gate-fail-cancel" autofocus @click="emit('update:open', false)">Cancel</Button>
          <Button type="submit" variant="destructive" :disabled="!canSubmit">
            {{ submitting ? 'Failing…' : 'Confirm fail' }}
          </Button>
        </DialogFooter>
      </form>
    </DialogContent>
  </Dialog>
</template>
