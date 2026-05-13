import { computed, ref } from 'vue';
import { defineStore } from 'pinia';
import type { JsonSchema } from '@jsonforms/core';

export type UserPromptEntry = {
  id: string;
  message: string;
  schema: JsonSchema;
  timeoutMs: number;
  receivedAt: number;
};

type UserPromptRequestPayload = {
  id: string;
  kind: 'elicitation';
  payload: {
    message?: unknown;
    requestedSchema?: unknown;
  };
  timeoutMs: number;
};

let listenerRegistered = false;

function toEntry(payload: UserPromptRequestPayload): UserPromptEntry | null {
  if (payload.kind !== 'elicitation' || typeof payload.id !== 'string') return null;
  const message =
    typeof payload.payload?.message === 'string'
      ? payload.payload.message
      : 'MCP server requests input';
  const schema =
    typeof payload.payload?.requestedSchema === 'object' && payload.payload.requestedSchema !== null
      ? payload.payload.requestedSchema as JsonSchema
      : ({ type: 'object', properties: {} } as JsonSchema);
  return {
    id: payload.id,
    message,
    schema,
    timeoutMs: Number.isFinite(payload.timeoutMs) ? payload.timeoutMs : 120_000,
    receivedAt: Date.now(),
  };
}

export const useUserPromptStore = defineStore('userPrompt', () => {
  const queue = ref<UserPromptEntry[]>([]);
  const front = computed(() => queue.value[0] ?? null);
  const open = computed(() => front.value !== null);

  function enqueue(payload: UserPromptRequestPayload): void {
    const entry = toEntry(payload);
    if (!entry || queue.value.some((queued) => queued.id === entry.id)) return;
    queue.value.push(entry);
  }

  function shift(id: string): void {
    const index = queue.value.findIndex((entry) => entry.id === id);
    if (index >= 0) queue.value.splice(index, 1);
  }

  async function submit(content: Record<string, unknown>): Promise<void> {
    const entry = front.value;
    if (!entry) return;
    await window.api.resolveUserPrompt({
      id: entry.id,
      response: { action: 'accept', content },
    });
    shift(entry.id);
  }

  async function cancel(): Promise<void> {
    const entry = front.value;
    if (!entry) return;
    await window.api.resolveUserPrompt({
      id: entry.id,
      response: { action: 'cancel' },
    });
    shift(entry.id);
  }

  function setupIpcListener(): void {
    if (listenerRegistered) return;
    listenerRegistered = true;
    window.api.onUserPromptRequest(enqueue);
  }

  return {
    front,
    open,
    setupIpcListener,
    submit,
    cancel,
  };
});
