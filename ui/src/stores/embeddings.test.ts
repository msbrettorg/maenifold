/* @vitest-environment jsdom */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';

const { callToolMock } = vi.hoisted(() => ({
  callToolMock: vi.fn(),
}));

vi.mock('../lib/mcp/client', async (importActual) => {
  const actual = await importActual<typeof import('../lib/mcp/client')>();

  return {
    ...actual,
    mcpHttpClient: {
      callTool: callToolMock,
    },
  };
});

import { useEmbeddingsStore } from './embeddings';

const activeModelStatus = {
  state: 'ready',
  modelId: 'all-MiniLM-L6-v2',
  installedLayout: 'single-file',
  message: 'ready',
};

const statusResponse = {
  loaded: true,
  activeModelId: 'all-MiniLM-L6-v2',
  activeModelStatus,
  modelInstalled: true,
  vec0: {
    extensionPath: 'C:/sqlite-vec0.dll',
    installed: true,
    rustLoadable: true,
    loadError: null,
    tableCreationProbe: {
      success: true,
      error: null,
    },
    vectorTablesReady: true,
    embeddingsPopulated: true,
    rebuildRecommended: false,
    message: 'vec0 ready',
    tables: {
      vecConcepts: {
        exists: true,
        ready: true,
        count: 10,
        error: null,
      },
      vecMemoryFiles: {
        exists: true,
        ready: true,
        count: 5,
        error: null,
      },
    },
  },
  model: {
    filename: 'all-MiniLM-L6-v2.onnx',
    dimensions: 384,
    maxTokens: 256,
  },
};

const statusResponseWithoutTables = {
  loaded: false,
  activeModelId: null,
  activeModelStatus: {
    state: 'error',
    modelId: null,
    installedLayout: null,
    message: 'database open failed',
  },
  modelInstalled: false,
  vec0: {
    extensionPath: null,
    installed: false,
    rustLoadable: false,
    loadError: 'db-open failure',
    tableCreationProbe: {
      success: false,
      error: 'db-open failure',
    },
    vectorTablesReady: false,
    embeddingsPopulated: false,
    rebuildRecommended: false,
    message: 'db-open failure',
  },
};

const catalogResponse = {
  models: [
    {
      id: 'all-MiniLM-L6-v2',
      name: 'all-MiniLM-L6-v2',
      dimensions: 384,
      maxTokens: 256,
      sizeBytes: 80_000_000,
      license: 'Apache-2.0',
      huggingfaceRepo: 'sentence-transformers/all-MiniLM-L6-v2',
      onnxRepo: 'Xenova/all-MiniLM-L6-v2',
      released: '2024-01-01',
      mrlDimensions: null,
      mtebRetrieval: 57.2,
      installed: true,
      installedLayout: 'single-file',
      isActive: true,
      downloadProgress: null,
    },
  ],
  currentDimensions: 384,
  activeModelId: 'all-MiniLM-L6-v2',
  activeModelStatus,
};

type GatewayArgs = {
  parameters?: {
    action?: string;
  };
};

type ToolCall = [string, GatewayArgs];

function gatewayTextResult(payload: unknown) {
  return {
    content: [{ type: 'text', text: JSON.stringify(payload) }],
  };
}

function mockSuccessfulActionResponses(): void {
  callToolMock.mockImplementation(async (_toolName: string, args: GatewayArgs) => {
    switch (args.parameters?.action) {
      case 'status':
        return gatewayTextResult(statusResponse);
      case 'list':
        return gatewayTextResult(catalogResponse);
      case 'activate':
      case 'download':
      case 'delete':
      case 'regenerate':
        return gatewayTextResult({ ok: true });
      default:
        throw new Error(`Unexpected action: ${String(args.parameters?.action)}`);
    }
  });
}

function countActionCalls(action: string): number {
  return (callToolMock.mock.calls as ToolCall[]).filter(([, args]) => args.parameters?.action === action)
    .length;
}

let store: ReturnType<typeof useEmbeddingsStore>;

beforeEach(() => {
  setActivePinia(createPinia());
  store = useEmbeddingsStore();
  callToolMock.mockReset();
});

afterEach(() => {
  store.stopPolling();
  vi.useRealTimers();
});

describe('useEmbeddingsStore', () => {
  it('fetchStatus parses the GatewayExecute envelope correctly', async () => {
    callToolMock.mockResolvedValueOnce(gatewayTextResult(statusResponse));

    await store.fetchStatus();

    expect(store.status?.vec0.installed).toBe(true);
  });

  it('fetchCatalog populates the catalog array', async () => {
    callToolMock.mockResolvedValueOnce(gatewayTextResult(catalogResponse));

    await store.fetchCatalog();

    expect(store.catalog.length).toBeGreaterThan(0);
  });

  it('fetchStatus tolerates status payloads without vec0.tables', async () => {
    callToolMock.mockResolvedValueOnce(gatewayTextResult(statusResponseWithoutTables));

    await store.fetchStatus();

    expect(store.status?.vec0.tables).toBeUndefined();
    expect(store.error).toBeNull();
  });

  it('action methods send the correct gateway payloads', async () => {
    mockSuccessfulActionResponses();

    await store.activate('all-MiniLM-L6-v2');

    expect(callToolMock).toHaveBeenNthCalledWith(1, 'GatewayExecute', {
      domain: 'maenifold.system',
      command: 'EmbeddingModels',
      parameters: {
        action: 'activate',
        modelId: 'all-MiniLM-L6-v2',
      },
    });

    callToolMock.mockClear();

    await store.download('all-MiniLM-L6-v2');

    expect(callToolMock).toHaveBeenNthCalledWith(1, 'GatewayExecute', {
      domain: 'maenifold.system',
      command: 'EmbeddingModels',
      parameters: {
        action: 'download',
        modelId: 'all-MiniLM-L6-v2',
      },
    });

    callToolMock.mockClear();

    await store.delete('all-MiniLM-L6-v2');

    expect(callToolMock).toHaveBeenNthCalledWith(1, 'GatewayExecute', {
      domain: 'maenifold.system',
      command: 'EmbeddingModels',
      parameters: {
        action: 'delete',
        modelId: 'all-MiniLM-L6-v2',
      },
    });
  });

  it('regenerate sends only the regenerate action payload', async () => {
    mockSuccessfulActionResponses();

    await store.regenerate();

    expect(callToolMock).toHaveBeenNthCalledWith(1, 'GatewayExecute', {
      domain: 'maenifold.system',
      command: 'EmbeddingModels',
      parameters: {
        action: 'regenerate',
      },
    });

    const firstCallParameters = (callToolMock.mock.calls[0]?.[1] as {
      parameters: Record<string, unknown>;
    }).parameters;
    expect(Object.prototype.hasOwnProperty.call(firstCallParameters, 'modelId')).toBe(false);
  });

  it('surfaces failures through error state and clears loading', async () => {
    callToolMock.mockRejectedValueOnce(new Error('daemon unreachable'));

    await expect(store.fetchStatus()).rejects.toThrow('daemon unreachable');

    expect(store.error).toBe('daemon unreachable');
    expect(store.loading).toBe(false);
  });

  it('starts and stops polling without scheduling duplicate refreshes after stop', async () => {
    vi.useFakeTimers();
    mockSuccessfulActionResponses();

    store.startPolling();

    await vi.advanceTimersByTimeAsync(12_000);

    expect(countActionCalls('status')).toBeGreaterThanOrEqual(2);

    store.stopPolling();
    const callCountAfterStop = callToolMock.mock.calls.length;

    await vi.advanceTimersByTimeAsync(12_000);

    expect(callToolMock).toHaveBeenCalledTimes(callCountAfterStop);
  });
});
