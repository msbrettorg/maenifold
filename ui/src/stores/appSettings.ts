import { ref } from 'vue';
import { defineStore } from 'pinia';

import type { AgentsConfig } from '../lib/types';
import { mcpHttpClient } from '../lib/mcp/client';
import {
  buildNestedPatch,
  extractToolText,
  isPlainObject,
  type JsonSchemaNode,
  type ToolCallTextResult,
} from '../lib/settingsSchema';

async function callGatewayCommand<T = unknown>(
  command: string,
  parameters: Record<string, unknown>
): Promise<T> {
  const result = await mcpHttpClient.callTool<ToolCallTextResult>('GatewayExecute', {
    domain: 'maenifold.system',
    command,
    parameters,
  });

  return JSON.parse(extractToolText(result)) as T;
}

export { buildNestedPatch };

export const useAppSettingsStore = defineStore('appSettings', () => {
  const schema = ref<object | null>(null);
  const values = ref<object | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);

  let pendingRequests = 0;

  function beginRequest(): void {
    if (pendingRequests === 0) {
      error.value = null;
    }

    pendingRequests += 1;
    loading.value = true;
  }

  function endRequest(): void {
    pendingRequests = Math.max(0, pendingRequests - 1);
    loading.value = pendingRequests > 0;
  }

  async function fetchSchema(): Promise<void> {
    beginRequest();

    try {
      const nextSchema = await callGatewayCommand<JsonSchemaNode>('GetSettingsSchema', {});
      schema.value = isPlainObject(nextSchema) ? nextSchema : null;
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err);
      throw err;
    } finally {
      endRequest();
    }
  }

  async function fetchConfig(): Promise<void> {
    beginRequest();

    try {
      const config = await callGatewayCommand<AgentsConfig>('GetConfig', {});
      values.value = isPlainObject(config.settings) ? config.settings : {};
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err);
      throw err;
    } finally {
      endRequest();
    }
  }

  async function setLeaf(path: string[], value: unknown): Promise<void> {
    if (path.length === 0) {
      throw new Error('setLeaf requires at least one path segment');
    }

    beginRequest();

    try {
      await callGatewayCommand<unknown>('SetSettings', {
        settings: buildNestedPatch(path, value),
      });
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err);
      throw err;
    } finally {
      endRequest();
    }

    await fetchConfig();
  }

  return {
    schema,
    values,
    loading,
    error,
    fetchSchema,
    fetchConfig,
    setLeaf,
  };
});
