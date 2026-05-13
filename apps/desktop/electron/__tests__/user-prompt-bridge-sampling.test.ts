// Failing TDD tests for sampling support on the user-prompt bridge (T-303.4.8).
// These reference the kind-tagged HTTP body shape the SWE will introduce.

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

describe('user prompt bridge — sampling kind', () => {
  beforeEach(() => {
    vi.useRealTimers();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('bridge_sends_kind_sampling_in_resolve_body', async () => {
    const { fetchMock, resolve } = await loadBridge();

    await expect(resolve({}, {
      kind: 'sampling',
      id: 'sampling-allow',
      response: { action: 'allow_once', selectedModel: 'gpt-5.5' },
    })).resolves.toEqual({ ok: true });

    expect(fetchMock).toHaveBeenCalledWith('http://localhost:8080/api/user-prompt/respond', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        kind: 'sampling',
        id: 'sampling-allow',
        response: { action: 'allow_once', selectedModel: 'gpt-5.5' },
      }),
    });
  });

  it('bridge_accepts_sampling_deny_action', async () => {
    const { fetchMock, resolve } = await loadBridge();

    await expect(resolve({}, {
      kind: 'sampling',
      id: 'sampling-deny',
      response: { action: 'deny' },
    })).resolves.toEqual({ ok: true });

    expect(fetchMock).toHaveBeenCalled();
  });

  it('bridge_rejects_payload_missing_kind', async () => {
    const { resolve } = await loadBridge();

    await expect(resolve({}, {
      id: 'no-kind',
      response: { action: 'accept' },
    })).rejects.toThrow();
  });

  it('bridge_rejects_unknown_kind', async () => {
    const { resolve } = await loadBridge();

    await expect(resolve({}, {
      kind: 'unknown',
      id: 'wat',
      response: { action: 'accept' },
    })).rejects.toThrow();
  });
});
