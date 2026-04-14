import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { AcpClientBridgeCore } from '../acp-client-core';
import { PermissionHandler } from '../acp-permissions';
import { useTrafficStore } from '../../stores/traffic';

class MockWebSocket {
  static readonly CONNECTING = 0;
  static readonly OPEN = 1;
  static readonly CLOSING = 2;
  static readonly CLOSED = 3;
  static instances: MockWebSocket[] = [];

  readonly url: string;
  readonly sent: string[] = [];
  readyState = MockWebSocket.CONNECTING;
  throwOnSend: Error | null = null;
  private listeners: Record<string, Array<{ handler: Function; once: boolean }>> = {};

  constructor(url: string | URL) {
    this.url = String(url);
    MockWebSocket.instances.push(this);
  }

  addEventListener(
    type: string,
    handler: Function,
    options?: { once?: boolean } | boolean,
  ): void {
    if (!this.listeners[type]) this.listeners[type] = [];
    const once = typeof options === 'object' ? !!options.once : false;
    this.listeners[type].push({ handler, once });
  }

  removeEventListener(type: string, handler: Function): void {
    const list = this.listeners[type];
    if (!list) return;
    this.listeners[type] = list.filter((entry) => entry.handler !== handler);
  }

  private emit(type: string, event: unknown): void {
    const list = this.listeners[type] ?? [];
    const kept: typeof list = [];
    for (const entry of list) {
      entry.handler(event);
      if (!entry.once) kept.push(entry);
    }
    this.listeners[type] = kept;
  }

  send(data: string): void {
    if (this.throwOnSend) {
      throw this.throwOnSend;
    }
    this.sent.push(data);
  }

  close(): void {
    this.readyState = MockWebSocket.CLOSED;
    this.emit('close', { code: 1000, reason: '', wasClean: true } as CloseEvent);
  }

  triggerOpen(): void {
    this.readyState = MockWebSocket.OPEN;
    this.emit('open', {} as Event);
  }

  triggerMessage(data: string): void {
    this.emit('message', { data } as MessageEvent);
  }

  triggerClose(code = 1011, reason = 'Agent process exited'): void {
    this.readyState = MockWebSocket.CLOSED;
    this.emit('close', { code, reason, wasClean: false } as CloseEvent);
  }
}

class TestAcpClientBridgeCore extends AcpClientBridgeCore {
  request<T>(method: string, params?: unknown): Promise<T> {
    return this.sendRequest<T>(method, params);
  }

  timeoutFor(method: string): number {
    return (
      this as unknown as {
        getTimeoutForMethod(method: string): number;
      }
    ).getTimeoutForMethod(method);
  }
}

describe('AcpClientBridgeCore request hardening', () => {
  const originalWebSocket = globalThis.WebSocket;

  function getLatestSocket(): MockWebSocket | undefined {
    return MockWebSocket.instances[MockWebSocket.instances.length - 1];
  }

  async function connectBridge(agentId = 'agent-1') {
    const bridge = new TestAcpClientBridgeCore(agentId, agentId, new PermissionHandler());
    const connectPromise = bridge.connect();
    const socket = getLatestSocket();
    if (!socket) {
      throw new Error('Expected WebSocket to be created');
    }
    socket.triggerOpen();
    await connectPromise;
    return { bridge, socket };
  }

  beforeEach(() => {
    vi.useFakeTimers();
    setActivePinia(createPinia());
    useTrafficStore().clear();
    MockWebSocket.instances = [];
    globalThis.WebSocket = MockWebSocket as unknown as typeof WebSocket;
    vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.spyOn(console, 'warn').mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.useRealTimers();
    globalThis.WebSocket = originalWebSocket;
  });

  it('connects over the ACP relay websocket path', async () => {
    const connectPromise = new TestAcpClientBridgeCore('agent-1', 'agent-1', new PermissionHandler()).connect();
    const socket = getLatestSocket();

    expect(socket?.url).toBe(
      'ws://localhost:8080/api/acp/session?agent=agent-1&keepSessionAlive=true'
    );

    socket?.triggerOpen();
    await expect(connectPromise).resolves.toBeUndefined();
  });

  it('clears request timeout handles after a successful response', async () => {
    const { bridge, socket } = await connectBridge();
    const promise = bridge.request<{ ok: boolean }>('initialize', {});

    expect(vi.getTimerCount()).toBe(1);

    socket.triggerMessage(JSON.stringify({ jsonrpc: '2.0', id: 0, result: { ok: true } }));

    await expect(promise).resolves.toEqual({ ok: true });
    expect(vi.getTimerCount()).toBe(0);
    expect(
      (
        bridge as unknown as {
          timeoutHandles: Map<number, ReturnType<typeof setTimeout>>;
        }
      ).timeoutHandles.size
    ).toBe(0);
  });

  it('clears request timeout handles after an error response', async () => {
    const { bridge, socket } = await connectBridge();
    const promise = bridge.request('initialize', {});

    expect(vi.getTimerCount()).toBe(1);

    socket.triggerMessage(
      JSON.stringify({
        jsonrpc: '2.0',
        id: 0,
        error: { code: -32000, message: 'backend failed' },
      })
    );

    await expect(promise).rejects.toThrow('backend failed');
    expect(vi.getTimerCount()).toBe(0);
    expect(
      (
        bridge as unknown as {
          timeoutHandles: Map<number, ReturnType<typeof setTimeout>>;
        }
      ).timeoutHandles.size
    ).toBe(0);
  });

  it('clears request timeout handles when websocket send fails', async () => {
    const { bridge, socket } = await connectBridge();
    socket.throwOnSend = new Error('send failed');

    const promise = bridge.request('initialize', {});

    await expect(promise).rejects.toThrow('send failed');
    expect(vi.getTimerCount()).toBe(0);
    expect(
      (
        bridge as unknown as {
          timeoutHandles: Map<number, ReturnType<typeof setTimeout>>;
        }
      ).timeoutHandles.size
    ).toBe(0);
  });

  it('rejects pending requests immediately on disconnect', async () => {
    const { bridge, socket } = await connectBridge();
    const promise = bridge.request('session/new', {});

    expect(vi.getTimerCount()).toBe(1);

    await bridge.disconnect();

    await expect(promise).rejects.toThrow('ACP client disconnected before response (agent-1)');
    expect(socket.readyState).toBe(MockWebSocket.CLOSED);
    expect(vi.getTimerCount()).toBe(0);
  });

  it('rejects pending requests when the websocket closes unexpectedly', async () => {
    const { bridge, socket } = await connectBridge();
    const promise = bridge.request('session/new', {});

    socket.triggerClose();

    await expect(promise).rejects.toThrow('ACP agent terminated before responding (agent-1)');
    expect(vi.getTimerCount()).toBe(0);
  });

  it('uses long-lived timeouts for ACP control mutations', () => {
    const bridge = new TestAcpClientBridgeCore('agent-1', 'agent-1', new PermissionHandler());

    expect(bridge.timeoutFor('session/set_config_option')).toBe(30 * 60 * 1000);
  });
});
