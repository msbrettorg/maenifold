<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import { useGateMcp } from '../composables/useGateMcp';
import type { GateTask } from '../lib/gate';
import { extractSprint, GATE_PIPELINES, GATE_PRIORITIES, nextTaskId } from '../lib/gate-pipelines';

const props = defineProps<{ open: boolean; root: string; tasks: GateTask[] }>();
const emit = defineEmits<{ 'update:open': [value: boolean]; created: [taskId: string] }>();

const gateMcp = useGateMcp();
const taskId = ref('');
const title = ref('');
const pipeline = ref('tdd');
const priority = ref('P0');
const sprint = ref(1);
const body = ref('');
const error = ref('');
const submitting = ref(false);

const pipelineOptions = [...GATE_PIPELINES];
const priorityOptions = [...GATE_PRIORITIES];
const existingIds = computed(() => new Set(props.tasks.map((task) => task.taskId)));
const idPattern = computed(() => new RegExp(`^T-${sprint.value}\\.[1-9]\\d*$`));

function initializeForm() {
  const id = nextTaskId(props.tasks);
  taskId.value = id;
  sprint.value = extractSprint(id) ?? 1;
  title.value = '';
  pipeline.value = 'tdd';
  priority.value = 'P0';
  body.value = '';
  error.value = '';
}

watch(() => props.open, (open) => {
  if (open) initializeForm();
}, { immediate: true });

function validate(): boolean {
  const id = taskId.value.trim();
  if (!idPattern.value.test(id)) {
    error.value = `Task id must match T-${sprint.value}.{n}.`;
    return false;
  }
  if (existingIds.value.has(id)) {
    error.value = `Task id ${id} already exists.`;
    return false;
  }
  if (!title.value.trim()) {
    error.value = 'Title is required.';
    return false;
  }
  if (!GATE_PIPELINES.includes(pipeline.value as typeof GATE_PIPELINES[number])) {
    error.value = 'Pipeline is not supported.';
    return false;
  }
  return true;
}

async function submit() {
  error.value = '';
  if (!validate()) return;
  submitting.value = true;
  try {
    const id = taskId.value.trim();
    const trimmedRoot = props.root.trim();
    const trimmedTitle = title.value.trim();
    const trimmedPipeline = pipeline.value.trim();
    const trimmedPriority = priority.value.trim();
    const trimmedBody = body.value.trim();
    await gateMcp.createTask({
      root: trimmedRoot,
      taskId: id,
      title: trimmedTitle,
      pipeline: trimmedPipeline,
      priority: trimmedPriority,
      sprint: sprint.value,
      ...(trimmedBody ? { body: trimmedBody } : {}),
    });
    emit('created', id);
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
    <DialogContent v-if="open" class="border-border-card bg-bg-surface text-text-primary sm:max-w-2xl">
      <DialogHeader>
        <DialogTitle>Create gate task</DialogTitle>
        <DialogDescription>Create a todo markdown file, then sync gate cache.</DialogDescription>
      </DialogHeader>
      <form class="grid gap-4" data-testid="gate-create-form" @submit.prevent="submit">
        <Alert v-if="error" variant="destructive">
          <AlertDescription>{{ error }}</AlertDescription>
        </Alert>
        <div class="grid grid-cols-2 gap-3">
          <div class="grid gap-2">
            <Label for="gate-create-id">ID</Label>
            <Input id="gate-create-id" v-model="taskId" data-testid="gate-create-id" />
          </div>
          <div class="grid gap-2">
            <Label for="gate-create-sprint">Sprint</Label>
            <Input id="gate-create-sprint" v-model.number="sprint" type="number" data-testid="gate-create-sprint" />
          </div>
        </div>
        <div class="grid gap-2">
          <Label for="gate-create-title">Title</Label>
          <Input id="gate-create-title" v-model="title" data-testid="gate-create-title" />
        </div>
        <div class="grid grid-cols-2 gap-3">
          <div class="grid gap-2">
            <Label>Pipeline</Label>
            <Select v-model="pipeline">
              <SelectTrigger><SelectValue placeholder="Pipeline" /></SelectTrigger>
              <SelectContent>
                <SelectItem v-for="option in pipelineOptions" :key="option" :value="option">{{ option }}</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div class="grid gap-2">
            <Label>Priority</Label>
            <Select v-model="priority">
              <SelectTrigger><SelectValue placeholder="Priority" /></SelectTrigger>
              <SelectContent>
                <SelectItem v-for="option in priorityOptions" :key="option" :value="option">{{ option }}</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>
        <div class="grid gap-2">
          <Label for="gate-create-body">Body</Label>
          <Textarea id="gate-create-body" v-model="body" data-testid="gate-create-body" class="min-h-40" />
        </div>
        <DialogFooter>
          <Button type="button" variant="ghost" @click="emit('update:open', false)">Cancel</Button>
          <Button type="submit" :disabled="submitting">{{ submitting ? 'Creating…' : 'Create task' }}</Button>
        </DialogFooter>
      </form>
    </DialogContent>
  </Dialog>
</template>
