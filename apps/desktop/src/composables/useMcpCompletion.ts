import { computed } from 'vue';
import type {
  CompleteResult,
  PromptReference,
  ResourceTemplateReference,
} from '@modelcontextprotocol/sdk/types';
import { getSharedMcpClient } from './useSharedMcpClient';
import { McpCapabilityUnsupportedError } from './useMcpResources';

export type CompletionReference = PromptReference | ResourceTemplateReference;

export interface McpCompletionClient {
  complete: (ref: CompletionReference, argName: string, value: string) => Promise<CompleteResult>;
  supported: Readonly<{ value: boolean }>;
}

const sharedMcpCompletion = new Map<string, McpCompletionClient>();
const maxCompletionValues = 100;

function capCompletionResult(result: CompleteResult): CompleteResult {
  if (result.completion.values.length <= maxCompletionValues) {
    return result;
  }
  return {
    ...result,
    completion: {
      ...result.completion,
      values: result.completion.values.slice(0, maxCompletionValues),
      total: result.completion.total ?? result.completion.values.length,
      hasMore: true,
    },
  };
}

export function useMcpCompletion(baseUrl?: string): McpCompletionClient {
  const key = baseUrl ?? '__default__';
  const existing = sharedMcpCompletion.get(key);
  if (existing) {
    return existing;
  }

  const mcp = getSharedMcpClient(baseUrl);
  const supported = computed(() => mcp.supportsCompletion());

  async function ensureConnected(): Promise<void> {
    if (mcp.connected.value) {
      return;
    }
    await mcp.connect();
  }

  async function complete(
    ref: CompletionReference,
    argName: string,
    value: string,
  ): Promise<CompleteResult> {
    await ensureConnected();
    if (!mcp.supportsCompletion()) {
      throw new McpCapabilityUnsupportedError('completions');
    }
    return capCompletionResult(await mcp.complete(ref, argName, value));
  }

  const client: McpCompletionClient = {
    complete,
    supported,
  };

  sharedMcpCompletion.set(key, client);
  return client;
}
