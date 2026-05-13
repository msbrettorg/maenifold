import { computed, ref } from 'vue';
import { defineStore } from 'pinia';
import type { JsonSchema } from '@jsonforms/core';
import { useSamplingStore, type SamplingRequest } from './sampling';

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

type SamplingRequestPayload = {
  id: string;
  kind: 'sampling';
  payload: {
    messages?: unknown;
    modelPreferences?: unknown;
    systemPrompt?: unknown;
    maxTokens?: unknown;
    metadata?: unknown;
  };
  timeoutMs?: number;
};

type UserPromptRequestPayload = ElicitationRequestPayload | SamplingRequestPayload;

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

function textFromSamplingContent(content: unknown): string {
  if (typeof content === 'string') return content;
  if (Array.isArray(content)) {
    return content.map(textFromSamplingContent).filter(Boolean).join('\n');
  }
  if (typeof content !== 'object' || content === null) return '';
  const record = content as Record<string, unknown>;
  if (typeof record.text === 'string') return record.text;
  if (typeof record.type === 'string') return `[${record.type}]`;
  return '';
}

function modelHintFrom(payload: SamplingRequestPayload['payload']): string | undefined {
  const preferences = payload.modelPreferences;
  if (typeof preferences !== 'object' || preferences === null) return undefined;
  const hints = (preferences as { hints?: unknown }).hints;
  if (!Array.isArray(hints)) return undefined;
  const firstNamedHint = hints.find((hint) => {
    return typeof hint === 'object'
      && hint !== null
      && typeof (hint as { name?: unknown }).name === 'string';
  }) as { name?: string } | undefined;
  return firstNamedHint?.name;
}

function toSamplingRequest(payload: SamplingRequestPayload): SamplingRequest {
  const inner = (typeof payload.payload === 'object' && payload.payload !== null)
    ? payload.payload
    : ({} as SamplingRequestPayload['payload']);
  const messages = Array.isArray(inner.messages)
    ? inner.messages.flatMap((message) => {
        if (typeof message !== 'object' || message === null) return [];
        const record = message as Record<string, unknown>;
        return [{
          role: typeof record.role === 'string' ? record.role : 'user',
          text: textFromSamplingContent(record.content),
        }];
      })
    : [];
  return {
    id: payload.id,
    serverName: 'MCP server',
    modelHint: modelHintFrom(inner),
    messages,
    systemPrompt: typeof inner.systemPrompt === 'string'
      ? inner.systemPrompt
      : undefined,
    maxTokens: typeof inner.maxTokens === 'number' ? inner.maxTokens : 0,
    queuedAt: Date.now(),
  };
}

export const useUserPromptStore = defineStore('userPrompt', () => {
  const queue = ref<UserPromptEntry[]>([]);
  const front = computed(() => queue.value[0] ?? null);
  const open = computed(() => front.value !== null);

  function enqueue(payload: UserPromptRequestPayload): void {
    if (payload.kind === 'sampling') {
      const samplingStore = useSamplingStore();
      samplingStore.enqueue(toSamplingRequest(payload));
      return;
    }
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
    submit,
    cancel,
  };
});
