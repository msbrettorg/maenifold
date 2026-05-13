// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';

type UserPromptPayload = {
  id: string;
  kind: 'elicitation';
  payload: { message?: unknown; requestedSchema?: unknown };
  timeoutMs: number;
};

async function setup() {
  vi.resetModules();
  setActivePinia(createPinia());
  const resolveUserPrompt = vi.fn().mockResolvedValue({ ok: true });
  const listeners: Array<(payload: UserPromptPayload) => void> = [];
  Object.defineProperty(window, 'api', {
    value: {
      quit: vi.fn(),
      onUserPromptRequest: vi.fn((handler: (payload: UserPromptPayload) => void) => {
        listeners.push(handler);
        return vi.fn();
      }),
      resolveUserPrompt,
    },
    configurable: true,
  });
  const { useUserPromptStore } = await import('../user-prompt');
  const store = useUserPromptStore();
  store.setupIpcListener();
  const listener = listeners[0];
  if (!listener) throw new Error('user prompt listener was not registered');
  return { listener, resolveUserPrompt, store };
}

function promptPayload(id: string, message = 'Need approval'): UserPromptPayload {
  return {
    id,
    kind: 'elicitation',
    payload: {
      message,
      requestedSchema: {
        type: 'object',
        properties: { reason: { type: 'string' } },
        required: ['reason'],
      },
    },
    timeoutMs: 30_000,
  };
}

describe('useUserPromptStore', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    Reflect.deleteProperty(window, 'api');
    vi.spyOn(Date, 'now').mockReturnValue(1_700_000_000_000);
  });

  it('registers one IPC listener and opens on the first elicitation request', async () => {
    const { listener, store } = await setup();

    listener(promptPayload('prompt-1'));

    expect(window.api.onUserPromptRequest).toHaveBeenCalledTimes(1);
    expect(store.open).toBe(true);
    expect(store.front).toMatchObject({
      id: 'prompt-1',
      message: 'Need approval',
      timeoutMs: 30_000,
      receivedAt: 1_700_000_000_000,
    });
  });

  it('keeps duplicate ids out of the FIFO queue', async () => {
    const { listener, resolveUserPrompt, store } = await setup();
    listener(promptPayload('prompt-1', 'First'));
    listener(promptPayload('prompt-1', 'Duplicate'));
    listener(promptPayload('prompt-2', 'Second'));

    await store.cancel();

    expect(resolveUserPrompt).toHaveBeenCalledWith({
      kind: 'elicitation',
      id: 'prompt-1',
      response: { action: 'cancel' },
    });
    expect(store.front?.id).toBe('prompt-2');
  });

  it('submits the front prompt as an accepted MCP elicitation response', async () => {
    const { listener, resolveUserPrompt, store } = await setup();
    listener(promptPayload('prompt-accept'));

    await store.submit({ reason: 'approved' });

    expect(resolveUserPrompt).toHaveBeenCalledWith({
      kind: 'elicitation',
      id: 'prompt-accept',
      response: { action: 'accept', content: { reason: 'approved' } },
    });
    expect(store.open).toBe(false);
  });

  it('ignores non-elicitation payloads and defaults missing message/schema safely', async () => {
    const { listener, store } = await setup();
    listener({ ...promptPayload('bad'), kind: 'other' as UserPromptPayload['kind'] });
    expect(store.open).toBe(false);

    listener({
      id: 'prompt-defaults',
      kind: 'elicitation',
      payload: {},
      timeoutMs: Number.NaN,
    });

    expect(store.front).toMatchObject({
      id: 'prompt-defaults',
      message: 'MCP server requests input',
      schema: { type: 'object', properties: {} },
      timeoutMs: 120_000,
    });
  });
});
