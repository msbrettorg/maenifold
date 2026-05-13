// @vitest-environment jsdom
import type { CompleteResult, PromptReference } from '@modelcontextprotocol/sdk/types';
import { describe, expect, it, vi } from 'vitest';
import { createMcpClientOps, type SdkClient } from '../mcpClientOps';

const promptRef: PromptReference = { type: 'ref/prompt', name: 'completable-prompt' };

function setup() {
  const result: CompleteResult = {
    completion: {
      values: ['formal', 'friendly'],
      total: 2,
    },
  };
  const client = {
    complete: vi.fn(async () => result),
  } as unknown as SdkClient;
  const setError = vi.fn();
  const ops = createMcpClientOps({
    ensureConnectedSession: vi.fn(async () => client),
    reconnectSession: vi.fn(async () => undefined),
    setError,
  });
  return { client, ops, result, setError };
}

describe('mcpClientOps complete', () => {
  it('calls SDK complete with prompt ref and argument shape', async () => {
    const { client, ops, result, setError } = setup();

    await expect(ops.complete(promptRef, 'style', 'f')).resolves.toBe(result);

    expect(client.complete).toHaveBeenCalledTimes(1);
    expect(client.complete).toHaveBeenCalledWith({
      ref: promptRef,
      argument: { name: 'style', value: 'f' },
    });
    expect(setError).toHaveBeenCalledWith(null);
  });
});
