// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';

type ReasonPayload = { kind: string; used?: number; budget?: number } | string;

function samplingPayload(reason: ReasonPayload) {
  return {
    id: 'sampling-reason',
    kind: 'sampling' as const,
    payload: {
      serverName: 'research-server',
      messages: [
        {
          role: 'user',
          content: { type: 'text', text: 'Need sampling.' },
        },
      ],
      maxTokens: 128,
      reason,
    },
    timeoutMs: 300_000,
  };
}

async function setup() {
  vi.resetModules();
  setActivePinia(createPinia());
  const [{ useSamplingStore }, { useUserPromptStore }] = await Promise.all([
    import('../sampling'),
    import('../user-prompt'),
  ]);
  return {
    samplingStore: useSamplingStore(),
    userPromptStore: useUserPromptStore(),
  };
}

describe('user prompt sampling reason parsing', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('parses budgetExceeded reason from payload', async () => {
    const { samplingStore, userPromptStore } = await setup();

    userPromptStore.enqueue(
      samplingPayload({ kind: 'budgetExceeded', used: 5, budget: 5 }),
    );

    expect(samplingStore.queue[0].reason).toEqual({
      kind: 'budgetExceeded',
      used: 5,
      budget: 5,
    });
  });

  it('parses newRequest reason from payload', async () => {
    const { samplingStore, userPromptStore } = await setup();

    userPromptStore.enqueue(samplingPayload({ kind: 'newRequest' }));

    expect(samplingStore.queue[0].reason).toEqual({ kind: 'newRequest' });
  });

  it('treats malformed reason as undefined', async () => {
    const invalidKind = await setup();
    invalidKind.userPromptStore.enqueue(
      samplingPayload({ kind: 'invalid-discriminator' }),
    );

    expect(invalidKind.samplingStore.queue[0].reason).toBeUndefined();

    const nonObject = await setup();
    nonObject.userPromptStore.enqueue(samplingPayload('not-an-object'));

    expect(nonObject.samplingStore.queue[0].reason).toBeUndefined();
  });
});
