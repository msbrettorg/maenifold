// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import type { SamplingDecision, SamplingRequest } from '../sampling';

function request(id: string, text = 'Please approve model output'): SamplingRequest {
  return {
    id,
    serverName: 'research-server',
    modelHint: 'gpt-4.1',
    messages: [{ role: 'user', text }],
    systemPrompt: 'Be concise',
    maxTokens: 512,
    queuedAt: 1_700_000_000_000,
  };
}

async function setup() {
  vi.resetModules();
  setActivePinia(createPinia());
  const resolveUserPrompt = vi.fn().mockResolvedValue({ ok: true });
  Object.defineProperty(window, 'api', {
    value: {
      quit: vi.fn(),
      onUserPromptRequest: vi.fn(),
      resolveUserPrompt,
    },
    configurable: true,
  });
  const { useSamplingStore } = await import('../sampling');
  return { resolveUserPrompt, store: useSamplingStore() };
}

describe('useSamplingStore', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    Reflect.deleteProperty(window, 'api');
  });

  it('sampling_store_starts_empty', async () => {
    const { store } = await setup();

    expect(store.length).toBe(0);
    expect(store.current).toBeNull();
  });

  it('enqueue_adds_request_and_exposes_first_via_current', async () => {
    const { store } = await setup();
    const first = request('sampling-1');

    store.enqueue(first);

    expect(store.current?.id).toBe(first.id);
    expect(store.length).toBe(1);
  });

  it('enqueue_two_requests_preserves_FIFO', async () => {
    const { store } = await setup();
    store.enqueue(request('sampling-1', 'First'));
    store.enqueue(request('sampling-2', 'Second'));

    expect(store.current?.id).toBe('sampling-1');
    expect(store.length).toBe(2);
  });

  it('resolve_calls_electron_api_with_kind_sampling_discriminator', async () => {
    const { resolveUserPrompt, store } = await setup();
    const decision: SamplingDecision = {
      action: 'allow_once',
      selectedModel: 'gpt-4.1-mini',
      selectedAgent: 'research-agent',
    };
    store.enqueue(request('sampling-approve'));

    await store.resolve('sampling-approve', decision);

    expect(resolveUserPrompt).toHaveBeenCalledWith({
      kind: 'sampling',
      id: 'sampling-approve',
      response: decision,
    });
  });

  it('resolve_removes_request_from_queue', async () => {
    const { store } = await setup();
    store.enqueue(request('sampling-1'));
    store.enqueue(request('sampling-2'));

    await store.resolve('sampling-1', { action: 'allow_always' });

    expect(store.length).toBe(1);
    expect(store.current?.id).toBe('sampling-2');
  });

  it('dismissExpired_removes_from_queue_without_calling_api', async () => {
    const { resolveUserPrompt, store } = await setup();
    store.enqueue(request('sampling-expired'));

    store.dismissExpired('sampling-expired');

    expect(store.length).toBe(0);
    expect(store.current).toBeNull();
    expect(resolveUserPrompt).not.toHaveBeenCalled();
  });
});
