import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { AcpClientBridgeCore } from '../acp-client-core';
import { AcpClientBridge } from '../acp-client';
import { PermissionHandler } from '../acp-permissions';
import { useTerminalsStore } from '../../stores/useTerminalsStore';

const DEFAULT_OUTPUT_BYTE_LIMIT = 1_048_576;

class MockWebSocket {
  static readonly OPEN = 1;
  static readonly CLOSED = 3;
  static instances: MockWebSocket[] = [];

  readonly sent: string[] = [];
  readyState = 0;
  private listeners: Record<string, Array<{ handler: Function; once: boolean }>> = {};

  constructor(readonly url: string | URL) {
    MockWebSocket.instances.push(this);
  }

  addEventListener(type: string, handler: Function, options?: { once?: boolean } | boolean): void {
    if (!this.listeners[type]) this.listeners[type] = [];
    this.listeners[type].push({
      handler,
      once: typeof options === 'object' ? !!options.once : false,
    });
  }

  send(data: string): void {
    this.sent.push(data);
  }

  close(): void {
    this.readyState = MockWebSocket.CLOSED;
  }

  triggerOpen(): void {
    this.readyState = MockWebSocket.OPEN;
    this.emit('open', {} as Event);
  }

  triggerMessage(data: string): void {
    this.emit('message', { data } as MessageEvent);
  }

  private emit(type: string, event: unknown): void {
    const list = this.listeners[type] ?? [];
    this.listeners[type] = list.filter((entry) => {
      entry.handler(event);
      return !entry.once;
    });
  }
}

type JsonRpcTestFrame = {
  id?: string | number | null;
  method?: string;
};

async function readLatestSentFrame(socket: MockWebSocket): Promise<JsonRpcTestFrame> {
  await vi.waitFor(() => {
    expect(socket.sent.length).toBeGreaterThan(0);
  });
  return JSON.parse(socket.sent[socket.sent.length - 1]) as JsonRpcTestFrame;
}

describe('AcpClientBridgeCore terminal outputByteLimit propagation', () => {
  const originalWebSocket = globalThis.WebSocket;

  beforeEach(() => {
    setActivePinia(createPinia());
    MockWebSocket.instances = [];
    globalThis.WebSocket = MockWebSocket as unknown as typeof WebSocket;
  });

  afterEach(() => {
    vi.restoreAllMocks();
    globalThis.WebSocket = originalWebSocket;
  });

  it('createTerminal stores the IPC-returned default outputByteLimit when ACP omits it', async () => {
    const bridge = new AcpClientBridgeCore('agent-1', 'agent-1', new PermissionHandler());
    vi.stubGlobal('electronAPI', {
      terminal: {
        create: vi.fn().mockResolvedValue({
          terminalId: 'term-default',
          outputByteLimit: DEFAULT_OUTPUT_BYTE_LIMIT,
        }),
      },
    });

    await bridge.createTerminal({ sessionId: 'sess-1', command: 'npm' });

    expect(useTerminalsStore().terminals.get('term-default')?.outputByteLimit).toBe(DEFAULT_OUTPUT_BYTE_LIMIT);
  });

  it('terminalCreate stores the ACP-returned default outputByteLimit when params omit it', async () => {
    const bridge = new AcpClientBridge('agent-1', 'agent-1', new PermissionHandler());
    const connectPromise = bridge.connect();
    const socket = MockWebSocket.instances[0];
    socket.triggerOpen();
    await connectPromise;

    const promise = (bridge as unknown as {
      terminalCreate(params: { sessionId: string; command: string }): Promise<{ terminalId: string }>;
    }).terminalCreate({ sessionId: 'sess-1', command: 'npm' });
    const frame = await readLatestSentFrame(socket);
    expect(frame.method).toBe('terminal/create');
    socket.triggerMessage(JSON.stringify({
      jsonrpc: '2.0',
      id: frame.id,
      result: { terminalId: 'term-default', outputByteLimit: DEFAULT_OUTPUT_BYTE_LIMIT },
    }));
    await promise;

    expect(useTerminalsStore().terminals.get('term-default')?.outputByteLimit).toBe(DEFAULT_OUTPUT_BYTE_LIMIT);
  });

  it('terminalCreate stores the ACP-returned explicit outputByteLimit', async () => {
    const bridge = new AcpClientBridge('agent-1', 'agent-1', new PermissionHandler());
    const connectPromise = bridge.connect();
    const socket = MockWebSocket.instances[0];
    socket.triggerOpen();
    await connectPromise;

    const promise = (bridge as unknown as {
      terminalCreate(params: {
        sessionId: string;
        command: string;
        outputByteLimit: number;
      }): Promise<{ terminalId: string }>;
    }).terminalCreate({
      sessionId: 'sess-1',
      command: 'npm',
      outputByteLimit: 50_000_000,
    });
    const frame = await readLatestSentFrame(socket);
    expect(frame.method).toBe('terminal/create');
    socket.triggerMessage(JSON.stringify({
      jsonrpc: '2.0',
      id: frame.id,
      result: { terminalId: 'term-explicit', outputByteLimit: 50_000_000 },
    }));
    await promise;

    expect(useTerminalsStore().terminals.get('term-explicit')?.outputByteLimit).toBe(50_000_000);
  });
});
