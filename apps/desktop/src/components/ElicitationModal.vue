<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { Button } from './ui/button';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from './ui/dialog';
import { Textarea } from './ui/textarea';
import SchemaFormRenderer from './SchemaFormRenderer.vue';
import { inspectFormSchema, normalizeFormData, stringifyRecord } from '../lib/schema-form';
import { useUserPromptStore } from '../stores/user-prompt';

const userPromptStore = useUserPromptStore();
const { front, open } = storeToRefs(userPromptStore);
const formData = ref<Record<string, unknown>>({});
const rawJson = ref('{}');
const rawError = ref('');
const now = ref(Date.now());

const timer = window.setInterval(() => {
  now.value = Date.now();
}, 1_000);

onBeforeUnmount(() => {
  window.clearInterval(timer);
});

const schemaAssessment = computed(() => inspectFormSchema(front.value?.schema));
const formSchema = computed(() => schemaAssessment.value.formSchema);
const fallbackReason = computed(() => schemaAssessment.value.fallbackReason);
const showRawFallback = computed(() => fallbackReason.value !== null);
const remainingSeconds = computed(() => {
  const entry = front.value;
  if (!entry) return 0;
  return Math.max(0, Math.ceil((entry.receivedAt + entry.timeoutMs - now.value) / 1_000));
});

watch(
  () => front.value?.id,
  () => {
    formData.value = {};
    rawJson.value = '{}';
    rawError.value = '';
  },
  { immediate: true },
);

function handleFormChange(event: { data: unknown }): void {
  formData.value = normalizeFormData(event.data);
}

function handleRawInput(value: string): void {
  rawJson.value = value;
  try {
    const parsed = JSON.parse(value);
    formData.value = normalizeFormData(parsed);
    rawError.value = '';
  } catch (error) {
    rawError.value = error instanceof Error ? error.message : 'Invalid JSON';
  }
}

async function handleSubmit(): Promise<void> {
  if (showRawFallback.value) {
    handleRawInput(rawJson.value);
    if (rawError.value) return;
  }
  await userPromptStore.submit(formData.value);
}
</script>

<template>
  <Dialog :open="open" @update:open="(nextOpen) => !nextOpen && userPromptStore.cancel()">
    <DialogContent class="gap-4 p-0 sm:max-w-[36rem]" :show-close-button="false">
      <DialogHeader class="px-6 pt-6">
        <div class="flex items-start justify-between gap-3">
          <div class="min-w-0">
            <DialogTitle>MCP user input required</DialogTitle>
            <p class="m-0 mt-2 text-sm text-text-secondary">{{ front?.message }}</p>
          </div>
          <span class="shrink-0 rounded-[12px] bg-bg-elevated px-2 py-1 text-xs text-text-secondary">
            {{ remainingSeconds }}s
          </span>
        </div>
      </DialogHeader>

      <div class="px-6">
        <div v-if="showRawFallback" class="grid gap-3">
          <p class="m-0 text-xs text-text-secondary">
            This schema uses features that the Draft-07 renderer may not support, so enter JSON
            directly. The daemon validates the response before it reaches the MCP server.
          </p>
          <pre class="max-h-40 overflow-auto rounded-[8px] bg-bg-elevated p-3 text-xs text-text-secondary">{{ stringifyRecord(front?.schema ?? {}) }}</pre>
          <Textarea :model-value="rawJson" rows="8" @update:model-value="handleRawInput(String($event))" />
          <p v-if="rawError" class="m-0 text-xs text-danger">{{ rawError }}</p>
        </div>
        <SchemaFormRenderer
          v-else
          ariaLabel="MCP elicitation form"
          :data="formData"
          :schema="formSchema ?? undefined"
          @change="handleFormChange"
        />
      </div>

      <DialogFooter class="gap-2 px-6 pb-6">
        <Button type="button" variant="ghost" @click="userPromptStore.cancel()">Cancel</Button>
        <Button type="button" :disabled="Boolean(rawError)" @click="handleSubmit()">Submit</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
