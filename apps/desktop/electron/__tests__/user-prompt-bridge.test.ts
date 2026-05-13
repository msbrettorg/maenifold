import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  handlers: new Map<string, (event: unknown, payload: unknown) => unknown>(),
}));

vi.mock('electron', () => ({
  ipcMain: {
    handle: vi.fn((channel: string, handler: (event: unknown, payload: unknown) => unknown) => {
      mocks.handlers.set(channel, handler);
    }),
  },
}));

class FakeEventSource {
  static instances: FakeEventSource[] = [];
  readonly url: string;
  onmessage: ((event: MessageEvent) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  readonly close = vi.fn();
  private readonly listeners = new Map<string, (event: MessageEvent) => void>();

  constructor(url: string) {
    this.url = url;
    FakeEventSource.instances.push(this);
  }

  addEventListener(type: string, listener: EventListener): void {
    this.listeners.set(type, listener as (event: MessageEvent) => void);
  }

  emitMessage(data: string): void {
    this.onmessage?.({ data } as MessageEvent);
  }

  emitUserPrompt(data: string): void {
    this.listeners.get('user-prompt')?.({ data } as MessageEvent);
  }
}

async function loadBridge() {
  vi.resetModules();
  mocks.handlers.clear();
  FakeEventSource.instances = [];
  vi.stubGlobal('EventSource', FakeEventSource);
  const fetchMock = vi.fn().mockResolvedValue({ ok: true, status: 200 });
  vi.stubGlobal('fetch', fetchMock);
  const webContents = {
    isDestroyed: vi.fn(() => false),
    send: vi.fn(),
  };
  const bridge = await import('../user-prompt-bridge');
  bridge.registerUserPromptBridge(() => webContents as never);
  const resolve = mocks.handlers.get('mcp:user-prompt:resolve');
  if (!resolve) throw new Error('resolve handler was not registered');
  return { fetchMock, resolve, webContents };
}

describe('user prompt bridge', () => {
  beforeEach(() => {
    vi.useRealTimers();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('forwards SSE user-prompt events to the renderer process', async () => {
    const { webContents } = await loadBridge();
    const source = FakeEventSource.instances[0];

    expect(source.url).toBe('http://localhost:8080/api/user-prompt/stream');
    source.emitUserPrompt('{"id":"prompt-1","kind":"elicitation"}');

    expect(webContents.send).toHaveBeenCalledWith('mcp:user-prompt:request', {
      id: 'prompt-1',
      kind: 'elicitation',
    });
  });

  it('also forwards default EventSource message events', async () => {
    const { webContents } = await loadBridge();

    FakeEventSource.instances[0].emitMessage('{"id":"prompt-2"}');

    expect(webContents.send).toHaveBeenCalledWith('mcp:user-prompt:request', { id: 'prompt-2' });
  });

  it('posts accepted renderer responses to the daemon respond endpoint', async () => {
    const { fetchMock, resolve } = await loadBridge();

    await expect(resolve({}, {
      id: 'prompt-accept',
      response: { action: 'accept', content: { reason: 'approved' } },
    })).resolves.toEqual({ ok: true });

    expect(fetchMock).toHaveBeenCalledWith('http://localhost:8080/api/user-prompt/respond', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        id: 'prompt-accept',
        response: { action: 'accept', content: { reason: 'approved' } },
      }),
    });
  });

  it('surfaces duplicate 410 responses as a clear resolver error', async () => {
    const { fetchMock, resolve } = await loadBridge();
    fetchMock.mockResolvedValueOnce({ ok: false, status: 410 });

    await expect(resolve({}, {
      id: 'prompt-duplicate',
      response: { action: 'cancel' },
    })).rejects.toThrow('user prompt response failed with 410');
  });

  it('rejects malformed renderer response payloads before posting', async () => {
    const { fetchMock, resolve } = await loadBridge();

    await expect(resolve({}, {
      id: '',
      response: { action: 'accept', content: {} },
    })).rejects.toThrow('invalid user prompt id');
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
