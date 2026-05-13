<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import type { JsonSchema } from '@jsonforms/core';
import type { GetPromptResult, PromptArgument, PromptMessage } from '@modelcontextprotocol/sdk/types';
import { Button } from './ui/button';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from './ui/dialog';
import SchemaFormRenderer from './SchemaFormRenderer.vue';
import type { AvailableCommand } from '../lib/types';
import { getMcpPromptCommandMeta } from '../composables/useMcpPromptCommands';
import { useMcpPrompts } from '../composables/useMcpPrompts';

const props = defineProps<{
  command: AvailableCommand | null;
  open: boolean;
}>();

const emit = defineEmits<{
  'update:open': [open: boolean];
  resolved: [messages: PromptMessage[]];
  error: [message: string];
}>();

const prompts = useMcpPrompts();
const formValues = ref<Record<string, unknown>>({});
const isSubmitting = ref(false);
const meta = computed(() => props.command ? getMcpPromptCommandMeta(props.command) : null);
const title = computed(() => meta.value ? `MCP prompt: ${meta.value.promptName}` : 'MCP prompt');
const argumentSchema = computed<JsonSchema>(() => createArgumentSchema(meta.value?.arguments ?? []));

watch(
  () => props.open,
  (open) => {
    if (open) {
      formValues.value = {};
      isSubmitting.value = false;
    }
  },
);

function createArgumentSchema(args: PromptArgument[]): JsonSchema {
  const properties: Record<string, unknown> = {};
  const required: string[] = [];
  for (const arg of args) {
    properties[arg.name] = {
      type: 'string',
      title: arg.name,
      description: arg.description,
    };
    if (arg.required) required.push(arg.name);
  }
  return {
    type: 'object',
    properties,
    required,
  } as JsonSchema;
}

function handleFormChange(event: { data: unknown }): void {
  formValues.value = event.data && typeof event.data === 'object'
    ? event.data as Record<string, unknown>
    : {};
}

async function submitPrompt(): Promise<void> {
  if (!meta.value || isSubmitting.value) return;
  isSubmitting.value = true;
  try {
    const args = Object.fromEntries(
      Object.entries(formValues.value).filter(([, value]) => value !== undefined && value !== '')
    );
    const result: GetPromptResult = await prompts.getPrompt(meta.value.promptName, args as Record<string, string>);
    emit('resolved', result.messages);
    emit('update:open', false);
  } catch (error) {
    emit('error', error instanceof Error ? error.message : 'Failed to resolve MCP prompt.');
  } finally {
    isSubmitting.value = false;
  }
}
</script>

<template>
  <Dialog :open="open" @update:open="emit('update:open', $event)">
    <DialogContent class="gap-4 p-0 sm:max-w-[32rem]" :show-close-button="false">
      <DialogHeader class="px-6 pt-6">
        <DialogTitle>{{ title }}</DialogTitle>
        <p v-if="command?.description" class="m-0 text-xs text-text-secondary">
          {{ command.description }}
        </p>
      </DialogHeader>

      <div class="px-6">
        <SchemaFormRenderer
          :data="formValues"
          :schema="argumentSchema"
          aria-label="MCP prompt arguments"
          @change="handleFormChange"
        />
      </div>

      <DialogFooter class="gap-2 px-6 pb-6">
        <Button type="button" variant="ghost" @click="emit('update:open', false)">Cancel</Button>
        <Button type="button" :disabled="isSubmitting" @click="submitPrompt">
          Insert prompt
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
