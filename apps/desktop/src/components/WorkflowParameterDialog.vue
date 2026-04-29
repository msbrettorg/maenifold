<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import {
  coerceWorkflowInputValue,
  normalizeWorkflowInputs,
  type WorkflowInputField,
} from '../lib/workflow-inputs';

const props = defineProps<{
  open: boolean;
  inputs: unknown;
  error: string | null;
  submitting: boolean;
}>();

const emit = defineEmits<{
  'update:open': [open: boolean];
  submit: [values: Record<string, unknown>];
}>();

const fields = computed(() => normalizeWorkflowInputs(props.inputs));
const values = reactive<Record<string, string>>({});
const validationError = ref<string | null>(null);

function resetValues() {
  for (const key of Object.keys(values)) delete values[key];
  for (const field of fields.value) {
    values[field.name] = field.defaultValue;
  }
  validationError.value = null;
}

function inputType(field: WorkflowInputField): string {
  return field.type === 'number' ? 'number' : 'text';
}

function close() {
  emit('update:open', false);
}

function submit() {
  for (const field of fields.value) {
    if (field.required && !String(values[field.name] ?? '').trim()) {
      validationError.value = `${field.label} is required.`;
      return;
    }
  }
  validationError.value = null;
  const payload: Record<string, unknown> = {};
  for (const field of fields.value) {
    const value = coerceWorkflowInputValue(field, values[field.name] ?? '');
    if (value !== undefined) payload[field.name] = value;
  }
  emit('submit', payload);
}

watch(() => props.open, (open) => {
  if (open) resetValues();
}, { immediate: true });
</script>

<template>
  <Dialog :open="open" @update:open="emit('update:open', $event)">
    <DialogContent class="sm:max-w-[520px]">
      <DialogHeader>
        <DialogTitle>Run workflow</DialogTitle>
        <DialogDescription>
          Provide the required workflow parameters before starting execution.
        </DialogDescription>
      </DialogHeader>

      <div class="flex flex-col gap-4 py-2">
        <div v-for="field in fields" :key="field.name" class="flex flex-col gap-2">
          <Label :for="`workflow-input-${field.name}`">
            {{ field.label }}<span v-if="field.required" class="text-destructive"> *</span>
          </Label>
          <Textarea
            v-if="field.type === 'textarea'"
            :id="`workflow-input-${field.name}`"
            v-model="values[field.name]"
            :name="field.name"
            :aria-required="field.required"
          />
          <Select
            v-else-if="field.type === 'select' || field.type === 'boolean'"
            v-model="values[field.name]"
          >
            <SelectTrigger :id="`workflow-input-${field.name}`">
              <SelectValue placeholder="Select a value" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem
                v-for="option in field.type === 'boolean' ? ['true', 'false'] : field.options"
                :key="option"
                :value="option"
              >
                {{ option }}
              </SelectItem>
            </SelectContent>
          </Select>
          <Input
            v-else
            :id="`workflow-input-${field.name}`"
            v-model="values[field.name]"
            :name="field.name"
            :type="inputType(field)"
            :aria-required="field.required"
          />
          <p v-if="field.description" class="text-xs text-muted-foreground">
            {{ field.description }}
          </p>
        </div>
      </div>

      <p v-if="validationError || error" role="alert" class="text-sm text-destructive">
        {{ validationError || error }}
      </p>

      <DialogFooter>
        <Button type="button" variant="outline" @click="close">Cancel</Button>
        <Button
          type="button"
          :loading="submitting"
          data-testid="workflow-parameter-submit"
          @click="submit"
        >
          Run
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
