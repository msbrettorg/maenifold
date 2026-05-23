<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
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
import { useMcpCompletion } from '../composables/useMcpCompletion';
import { getMcpPromptCommandMeta, parseGetPromptResult } from '../composables/useMcpPromptCommands';
import { getSharedMcpClient } from '../composables/useSharedMcpClient';

const props = defineProps<{
  command: AvailableCommand | null;
  open: boolean;
}>();

const emit = defineEmits<{
  'update:open': [open: boolean];
  resolved: [messages: PromptMessage[]];
  error: [message: string];
}>();

const mcpClient = getSharedMcpClient();
const completion = useMcpCompletion();
const formValues = ref<Record<string, unknown>>({});
const isSubmitting = ref(false);
const activeArgName = ref<string | null>(null);
const completionValues = ref<string[]>([]);
const completionBusy = ref(false);
const meta = computed(() => props.command ? getMcpPromptCommandMeta(props.command) : null);
const title = computed(() => meta.value ? `MCP prompt: ${meta.value.promptName}` : 'MCP prompt');
const argumentSchema = computed<JsonSchema>(() => createArgumentSchema(meta.value?.arguments ?? []));
const completionSupported = computed(() => completion.supported.value);
const visibleCompletionValues = computed(() => completionValues.value.slice(0, 8));
let completionTimer: ReturnType<typeof setTimeout> | null = null;
let completionRequestId = 0;

watch(
  () => props.open,
  (open) => {
    clearCompletionState();
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

function clearCompletionState(): void {
  activeArgName.value = null;
  completionValues.value = [];
  completionBusy.value = false;
  completionRequestId += 1;
  if (completionTimer) {
    clearTimeout(completionTimer);
    completionTimer = null;
  }
}

function getChangedArg(previous: Record<string, unknown>, next: Record<string, unknown>): string | null {
  const names = meta.value?.arguments.map((arg) => arg.name) ?? [];
  return names.find((name) => previous[name] !== next[name]) ?? null;
}

function scheduleCompletion(argName: string, value: string): void {
  if (!meta.value || !completionSupported.value) return;
  if (completionTimer) clearTimeout(completionTimer);
  const requestId = completionRequestId + 1;
  completionRequestId = requestId;
  completionTimer = setTimeout(async () => {
    completionBusy.value = true;
    try {
      const result = await completion.complete(
        { type: 'ref/prompt', name: meta.value!.promptName },
        argName,
        value,
      );
      if (requestId === completionRequestId) {
        completionValues.value = result.completion.values;
      }
    } catch {
      if (requestId === completionRequestId) {
        completionValues.value = [];
      }
    } finally {
      if (requestId === completionRequestId) {
        completionBusy.value = false;
      }
    }
  }, 300);
}

function handleFormChange(event: { data: unknown }): void {
  const nextValues = event.data && typeof event.data === 'object'
    ? event.data as Record<string, unknown>
    : {};
  const changedArg = getChangedArg(formValues.value, nextValues);
  formValues.value = nextValues;
  if (!changedArg) return;
  activeArgName.value = changedArg;
  completionValues.value = [];
  scheduleCompletion(changedArg, String(nextValues[changedArg] ?? ''));
}

function applyCompletionSuggestion(value: string): void {
  if (!activeArgName.value) return;
  formValues.value = {
    ...formValues.value,
    [activeArgName.value]: value,
  };
  completionValues.value = [];
}

async function submitPrompt(): Promise<void> {
  if (!meta.value || isSubmitting.value) return;
  isSubmitting.value = true;
  try {
    const args = Object.fromEntries(
      Object.entries(formValues.value).filter(([, value]) => value !== undefined && value !== '')
    );
    if (!mcpClient.connected.value) await mcpClient.connect();
    const result: GetPromptResult = parseGetPromptResult(
      await mcpClient.getPrompt(meta.value.promptName, args as Record<string, string>),
    );
    emit('resolved', result.messages);
    emit('update:open', false);
  } catch (error) {
    emit('error', error instanceof Error ? error.message : 'Failed to resolve MCP prompt.');
  } finally {
    isSubmitting.value = false;
  }
}

onBeforeUnmount(clearCompletionState);
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
        <div
          v-if="completionSupported && activeArgName && (visibleCompletionValues.length > 0 || completionBusy)"
          class="mt-3 rounded-md bg-popover p-2 shadow-lg"
          role="listbox"
          :aria-label="`Completion suggestions for ${activeArgName}`"
        >
          <p class="m-0 px-2 pb-2 text-xs text-text-secondary">
            Suggestions for {{ activeArgName }}
          </p>
          <p v-if="completionBusy && visibleCompletionValues.length === 0" class="m-0 px-2 text-xs text-text-secondary">
            Loading suggestions…
          </p>
          <div v-else class="flex flex-col gap-1">
            <Button
              v-for="suggestion in visibleCompletionValues"
              :key="suggestion"
              type="button"
              variant="ghost"
              class="justify-start"
              role="option"
              @click="applyCompletionSuggestion(suggestion)"
            >
              {{ suggestion }}
            </Button>
          </div>
        </div>
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
