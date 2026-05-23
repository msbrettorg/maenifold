import type { GetPromptResult, Prompt as PromptDescriptor } from '@modelcontextprotocol/sdk/types';
import { getSharedMcpClient } from './useSharedMcpClient';
import { McpCapabilityUnsupportedError } from './useMcpResources';

export interface McpPromptsClient {
  listPrompts: () => Promise<PromptDescriptor[]>;
  getPrompt: (name: string, args?: Record<string, string>) => Promise<GetPromptResult>;
  onPromptListChanged: (handler: () => void) => () => void;
}

const sharedMcpPrompts = new Map<string, McpPromptsClient>();

export function useMcpPrompts(baseUrl?: string): McpPromptsClient {
  const key = baseUrl ?? '__default__';
  const existing = sharedMcpPrompts.get(key);
  if (existing) {
    return existing;
  }

  const mcp = getSharedMcpClient(baseUrl);
  const handlers = new Set<() => void>();

  mcp.onNotification((notification) => {
    if (notification.method !== 'notifications/prompts/list_changed') {
      return;
    }
    handlers.forEach((handler) => {
      handler();
    });
  });

  mcp.onDisconnect(() => {
    handlers.clear();
  });

  async function ensureConnected(): Promise<void> {
    if (mcp.connected.value) {
      return;
    }
    await mcp.connect();
  }

  async function listPrompts(): Promise<PromptDescriptor[]> {
    await ensureConnected();
    if (!mcp.supportsPrompts()) {
      throw new McpCapabilityUnsupportedError('prompts');
    }
    return mcp.listPrompts();
  }

  async function getPrompt(name: string, args?: Record<string, string>): Promise<GetPromptResult> {
    await ensureConnected();
    if (!mcp.supportsPrompts()) {
      throw new McpCapabilityUnsupportedError('prompts');
    }
    return mcp.getPrompt(name, args);
  }

  function onPromptListChanged(handler: () => void): () => void {
    handlers.add(handler);
    return () => {
      handlers.delete(handler);
    };
  }

  const client: McpPromptsClient = {
    listPrompts,
    getPrompt,
    onPromptListChanged,
  };

  sharedMcpPrompts.set(key, client);
  return client;
}
