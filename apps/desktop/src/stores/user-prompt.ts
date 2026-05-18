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

type ElicitationRequestPayload = {
  id: string;
  kind: 'elicitation';
  payload: {
    message?: unknown;
    requestedSchema?: unknown;
  };
  timeoutMs: number;
};

type UserPromptRequestPayload = ElicitationRequestPayload;

let listenerRegistered = false;

function toEntry(payload: ElicitationRequestPayload): UserPromptEntry | null {
  if (typeof payload.id !== 'string') return null;
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
    if (payload.kind !== 'elicitation') return;
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
      kind: 'elicitation',
      id: entry.id,
      response: { action: 'accept', content },
    });
    shift(entry.id);
  }

  async function cancel(): Promise<void> {
    const entry = front.value;
    if (!entry) return;
    await window.api.resolveUserPrompt({
      kind: 'elicitation',
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
    enqueue,
    submit,
    cancel,
  };
});
