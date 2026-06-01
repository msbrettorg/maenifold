import { defineStore } from 'pinia';
import { ref } from 'vue';

import { McpHttpError, mcpHttpClient } from '../lib/mcp/client';

const GATEWAY_TOOL_NAME = 'GatewayExecute';
const EMBEDDING_MODELS_DOMAIN = 'maenifold.system';
const EMBEDDING_MODELS_COMMAND = 'EmbeddingModels';
const GRAPH_DOMAIN = 'maenifold.graph';
const GRAPH_SYNC_COMMAND = 'Sync';
const DEFAULT_POLL_INTERVAL_MS = 6000;

type EmbeddingModelsAction =
  | 'status'
  | 'list'
  | 'download'
  | 'activate'
  | 'delete'
  | 'regenerate';

interface GatewayContentItem {
  type?: string;
  text?: string;
  [key: string]: unknown;
}

interface GatewayToolResult {
  content?: GatewayContentItem[];
  isError?: boolean;
  [key: string]: unknown;
}

interface EmbeddingModelsPayload {
  action: EmbeddingModelsAction;
  modelId?: string;
}

export interface ActiveModelStatus {
  state: string;
  modelId: string | null;
  installedLayout: string | null;
  message: string;
}

export interface EmbeddingTableStatus {
  exists: boolean;
  ready: boolean;
  count: number | null;
  error: string | null;
}

export interface EmbeddingVec0Status {
  extensionPath: string | null;
  installed: boolean;
  rustLoadable: boolean;
  loadError: string | null;
  tableCreationProbe: {
    success: boolean;
    error: string | null;
  };
  vectorTablesReady: boolean;
  embeddingsPopulated: boolean;
  rebuildRecommended: boolean;
  message: string;
  tables?: {
    vecConcepts: EmbeddingTableStatus;
    vecMemoryFiles: EmbeddingTableStatus;
  };
}

export interface EmbeddingRuntimeModel {
  filename: string;
  dimensions: number;
  maxTokens: number;
}

export interface EmbeddingStatus {
  loaded: boolean;
  activeModelId: string | null;
  activeModelStatus: ActiveModelStatus;
  modelInstalled: boolean;
  vec0: EmbeddingVec0Status;
  model?: EmbeddingRuntimeModel;
}

export interface EmbeddingDownloadProgress {
  current: number;
  total: number | null;
  filename: string | null;
}

export interface EmbeddingModel {
  id: string;
  name: string;
  dimensions: number;
  maxTokens: number;
  sizeBytes: number;
  license: string;
  huggingfaceRepo: string | null;
  onnxRepo: string | null;
  released: string | null;
  mrlDimensions: number[] | null;
  mtebRetrieval: number | null;
  installed: boolean;
  installedLayout: string | null;
  isActive: boolean;
  downloadProgress: EmbeddingDownloadProgress | null;
}

interface EmbeddingCatalogResponse {
  models: EmbeddingModel[];
  currentDimensions: number;
  activeModelId: string | null;
  activeModelStatus: ActiveModelStatus;
}

interface EmbeddingMutationResponse {
  [key: string]: unknown;
}

function formatError(error: unknown): string {
  if (error instanceof McpHttpError) {
    if (typeof error.bodyText === 'string' && error.bodyText.trim()) {
      return `${error.message}: ${error.bodyText}`.trim();
    }

    return error.message;
  }

  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}

function parseGatewayJson<T>(result: GatewayToolResult): T {
  const text = result.content?.find((item) => item.type === 'text' && typeof item.text === 'string')
    ?.text;

  if (typeof text !== 'string' || !text.trim()) {
    throw new Error('GatewayExecute EmbeddingModels did not return content[0].text');
  }

  if (result.isError) {
    throw new Error(text);
  }

  try {
    return JSON.parse(text) as T;
  } catch (error) {
    throw new Error(
      `Failed to parse EmbeddingModels JSON payload: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

export const useEmbeddingsStore = defineStore('embeddings', () => {
  const status = ref<EmbeddingStatus | null>(null);
  const catalog = ref<EmbeddingModel[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const pollIntervalMs = ref(DEFAULT_POLL_INTERVAL_MS);

  let pollTimer: ReturnType<typeof setInterval> | null = null;
  let refreshInFlight: Promise<void> | null = null;

  async function callEmbeddingModels<T>(payload: EmbeddingModelsPayload): Promise<T> {
    const result = await mcpHttpClient.callTool<GatewayToolResult>(GATEWAY_TOOL_NAME, {
      domain: EMBEDDING_MODELS_DOMAIN,
      command: EMBEDDING_MODELS_COMMAND,
      parameters: payload,
    });

    return parseGatewayJson<T>(result);
  }

  async function callGraphSync(): Promise<string> {
    const result = await mcpHttpClient.callTool<GatewayToolResult>(GATEWAY_TOOL_NAME, {
      domain: GRAPH_DOMAIN,
      command: GRAPH_SYNC_COMMAND,
      parameters: {},
    });

    const text = result.content?.find((item) => item.type === 'text' && typeof item.text === 'string')?.text;
    if (typeof text !== 'string' || !text.trim()) {
      throw new Error('GatewayExecute graph Sync did not return content[0].text');
    }
    if (result.isError) {
      throw new Error(text);
    }
    return text;
  }

  async function loadStatus(): Promise<EmbeddingStatus> {
    const nextStatus = await callEmbeddingModels<EmbeddingStatus>({ action: 'status' });
    status.value = nextStatus;
    return nextStatus;
  }

  async function loadCatalog(): Promise<EmbeddingCatalogResponse> {
    const response = await callEmbeddingModels<EmbeddingCatalogResponse>({ action: 'list' });
    catalog.value = response.models;
    return response;
  }

  async function refreshInternal(): Promise<void> {
    await loadStatus();
    await loadCatalog();
  }

  async function runWithState<T>(operation: () => Promise<T>): Promise<T> {
    loading.value = true;
    error.value = null;

    try {
      return await operation();
    } catch (nextError) {
      error.value = formatError(nextError);
      throw nextError;
    } finally {
      loading.value = false;
    }
  }

  async function fetchStatus(): Promise<EmbeddingStatus> {
    return runWithState(loadStatus);
  }

  async function fetchCatalog(): Promise<EmbeddingModel[]> {
    return runWithState(async () => {
      const response = await loadCatalog();
      return response.models;
    });
  }

  async function refresh(): Promise<void> {
    if (refreshInFlight) {
      return refreshInFlight;
    }

    refreshInFlight = runWithState(refreshInternal).finally(() => {
      refreshInFlight = null;
    });

    return refreshInFlight;
  }

  async function download(modelId: string): Promise<EmbeddingMutationResponse> {
    return runWithState(async () => {
      const response = await callEmbeddingModels<EmbeddingMutationResponse>({
        action: 'download',
        modelId,
      });
      await refreshInternal();
      return response;
    });
  }

  async function activate(modelId: string): Promise<EmbeddingMutationResponse> {
    return runWithState(async () => {
      const response = await callEmbeddingModels<EmbeddingMutationResponse>({
        action: 'activate',
        modelId,
      });
      await refreshInternal();
      return response;
    });
  }

  async function deleteModel(modelId: string): Promise<EmbeddingMutationResponse> {
    return runWithState(async () => {
      const response = await callEmbeddingModels<EmbeddingMutationResponse>({
        action: 'delete',
        modelId,
      });
      await refreshInternal();
      return response;
    });
  }

  async function regenerate(): Promise<EmbeddingMutationResponse> {
    return runWithState(async () => {
      const response = await callEmbeddingModels<EmbeddingMutationResponse>({
        action: 'regenerate',
      });
      // Regenerate alone only clears embedding metadata. Immediately fire
      // graph Sync so the daemon actually rebuilds vectors instead of
      // leaving the user with stale state and a "run sync yourself" hint.
      await callGraphSync();
      await refreshInternal();
      return response;
    });
  }

  function startPolling(): void {
    if (pollTimer) {
      return;
    }

    pollTimer = setInterval(() => {
      if (loading.value) {
        return;
      }

      void refresh().catch(() => undefined);
    }, pollIntervalMs.value);
  }

  function stopPolling(): void {
    if (!pollTimer) {
      return;
    }

    clearInterval(pollTimer);
    pollTimer = null;
  }

  return {
    status,
    catalog,
    loading,
    error,
    pollIntervalMs,
    fetchStatus,
    fetchCatalog,
    refresh,
    download,
    activate,
    delete: deleteModel,
    regenerate,
    startPolling,
    stopPolling,
  };
});
