import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { nextTick, type ComputedRef } from 'vue';
import { createAcpStoreSinks } from '../../composables/useAcpStoreSink';
import { AcpClientBridgeCore } from '../acp-client-core';
import { AcpClientBridge } from '../acp-client';
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

type TerminalState = {
  terminalId: string;
  output: string;
  truncated: boolean;
  released: boolean;
  exitStatus?: {
    exitCode?: number | null;
    signal?: string | null;
  } | null;
};

type UseTerminalsStoreModule = {
  useTerminalsStore: () => {
    setTerminal: (terminalId: string, state: TerminalState) => void;
    getTerminalState: (terminalId: string) => ComputedRef<TerminalState | null>;
  };
};

type TerminalBridgeMethods = {
  terminalCreate?: (params: {
    sessionId: string;
    command: string;
    args?: string[];
    env?: Array<{ name: string; value: string }>;
    cwd?: string;
    outputByteLimit?: number;
  }) => Promise<{ terminalId: string }>;
  terminalOutput?: (params: {
    sessionId: string;
    terminalId: string;
  }) => Promise<{
    output: string;
    truncated: boolean;
    exitStatus?: { exitCode: number | null; signal: string | null };
  }>;
  terminalWaitForExit?: (params: {
    sessionId: string;
    terminalId: string;
  }) => Promise<{ exitCode: number | null; signal: string | null }>;
  terminalKill?: (params: { sessionId: string; terminalId: string }) => Promise<unknown>;
  terminalRelease?: (params: { sessionId: string; terminalId: string }) => Promise<unknown>;
};

type JsonRpcTestFrame = {
  id?: string | number | null;
  method?: string;
  params?: unknown;
};

async function loadUseTerminalsStoreModule(): Promise<UseTerminalsStoreModule> {
  return vi.importActual<UseTerminalsStoreModule>('../../stores/useTerminalsStore');
}

function createTestSinks() {
  return createAcpStoreSinks();
}

describe('AcpClientBridgeCore request hardening', () => {
  const originalWebSocket = globalThis.WebSocket;

  function getLatestSocket(): MockWebSocket | undefined {
    return MockWebSocket.instances[MockWebSocket.instances.length - 1];
  }

  async function connectBridge(agentId = 'agent-1') {
    const bridge = new TestAcpClientBridgeCore(
      agentId,
      agentId,
      new PermissionHandler(),
      createTestSinks()
    );
    const connectPromise = bridge.connect();
    const socket = getLatestSocket();
    if (!socket) {
      throw new Error('Expected WebSocket to be created');
    }
    socket.triggerOpen();
    await connectPromise;
    return { bridge, socket };
  }

  async function connectAcpBridge(agentId = 'agent-1') {
    const bridge = new AcpClientBridge(agentId, agentId, new PermissionHandler(), createTestSinks());
    const connectPromise = bridge.connect();
    const socket = getLatestSocket();
    if (!socket) {
      throw new Error('Expected WebSocket to be created');
    }
    socket.triggerOpen();
    await connectPromise;
    return { bridge, socket };
  }

  async function readSentFrame(socket: MockWebSocket, index: number): Promise<JsonRpcTestFrame> {
    await vi.waitFor(() => {
      expect(socket.sent.length).toBeGreaterThan(index);
    });
    return JSON.parse(socket.sent[index]) as JsonRpcTestFrame;
  }

  async function readLatestSentFrame(socket: MockWebSocket): Promise<JsonRpcTestFrame> {
    await vi.waitFor(() => {
      expect(socket.sent.length).toBeGreaterThan(0);
    });
    return JSON.parse(socket.sent[socket.sent.length - 1]) as JsonRpcTestFrame;
  }

  function respondWithResult(socket: MockWebSocket, frame: JsonRpcTestFrame, result: unknown): void {
    socket.triggerMessage(JSON.stringify({ jsonrpc: '2.0', id: frame.id, result }));
  }

  function respondWithError(socket: MockWebSocket, frame: JsonRpcTestFrame, message: string): void {
    socket.triggerMessage(JSON.stringify({
      jsonrpc: '2.0',
      id: frame.id,
      error: { code: -32000, message },
    }));
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
    const connectPromise = new TestAcpClientBridgeCore(
      'agent-1',
      'agent-1',
      new PermissionHandler(),
      createTestSinks()
    ).connect();
    const socket = getLatestSocket();

    expect(socket?.url).toBe(
      'ws://localhost:8080/api/acp/session?agent=agent-1&keepSessionAlive=true'
    );

    socket?.triggerOpen();
    await expect(connectPromise).resolves.toBeUndefined();
  });

  it('records outbound request traffic entries through the injected sink', async () => {
    const trafficStore = useTrafficStore();
    const { bridge, socket } = await connectBridge();
    const params = { cwd: '/repo', mcpServers: [] };
    const promise = bridge.request<{ sessionId: string }>('session/new', params);
    const frame = await readSentFrame(socket, 0);

    expect(trafficStore.entries).toHaveLength(1);
    expect(trafficStore.entries[0]).toMatchObject({
      direction: 'out',
      type: 'request',
      method: 'session/new',
      requestId: frame.id,
      payload: {
        method: 'session/new',
        params: expect.objectContaining(params),
      },
    });

    respondWithResult(socket, frame, { sessionId: 'sess-1' });
    await expect(promise).resolves.toEqual({ sessionId: 'sess-1' });
  });

  it('records inbound response traffic entries with request id and error shape', async () => {
    const trafficStore = useTrafficStore();
    const { bridge, socket } = await connectBridge();
    const promise = bridge.request('initialize', {});
    const frame = await readSentFrame(socket, 0);

    respondWithError(socket, frame, 'backend failed');

    await expect(promise).rejects.toThrow('backend failed');
    expect(trafficStore.entries).toHaveLength(2);
    expect(trafficStore.entries[1]).toMatchObject({
      direction: 'in',
      type: 'response',
      method: 'initialize',
      requestId: frame.id,
      error: true,
      payload: {
        id: frame.id,
        error: { code: -32000, message: 'backend failed' },
      },
    });
  });

  it('records inbound notification traffic entries through the injected sink', async () => {
    const trafficStore = useTrafficStore();
    const { bridge, socket } = await connectBridge();
    const notification = {
      jsonrpc: '2.0',
      method: 'session/update',
      params: {
        sessionId: 'session-1',
        update: {
          sessionUpdate: 'available_commands_update',
          availableCommands: null,
        },
      },
    };
    bridge.onSessionUpdate = vi.fn();

    socket.triggerMessage(JSON.stringify(notification));

    expect(trafficStore.entries).toHaveLength(1);
    expect(trafficStore.entries[0]).toMatchObject({
      direction: 'in',
      type: 'notification',
      method: 'session/update',
      payload: notification,
    });
  });

  it('clears request timeout handles after a successful response', async () => {
    const { bridge, socket } = await connectBridge();
    const promise = bridge.request<{ ok: boolean }>('initialize', {});
    const frame = await readSentFrame(socket, 0);

    expect(vi.getTimerCount()).toBe(1);

    respondWithResult(socket, frame, { ok: true });

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
    const frame = await readSentFrame(socket, 0);

    expect(vi.getTimerCount()).toBe(1);

    respondWithError(socket, frame, 'backend failed');

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

  it('sends session/close before an explicit disconnect closes the relay socket', async () => {
    const { bridge, socket } = await connectBridge();

    await bridge.disconnect();

    expect(JSON.parse(socket.sent[0] ?? '{}')).toMatchObject({
      method: 'session/close',
      params: { _meta: expect.any(Object) },
    });
    expect(socket.readyState).toBe(MockWebSocket.CLOSED);
  });

  it('includes the active session id in explicit disconnect close', async () => {
    const { bridge, socket } = await connectAcpBridge();
    const newSession = bridge.newSession({ cwd: '/repo', mcpServers: [] });
    const frame = await readSentFrame(socket, 0);

    respondWithResult(socket, frame, { sessionId: 'sess-1' });
    await expect(newSession).resolves.toEqual({ sessionId: 'sess-1' });

    await bridge.disconnect();

    expect(JSON.parse(socket.sent[1] ?? '{}')).toMatchObject({
      method: 'session/close',
      params: { sessionId: 'sess-1' },
    });
  });

  it('rejects pending requests when the websocket closes unexpectedly', async () => {
    const { bridge, socket } = await connectBridge();
    const promise = bridge.request('session/new', {});

    socket.triggerClose();

    await expect(promise).rejects.toThrow('ACP agent terminated before responding (agent-1)');
    expect(vi.getTimerCount()).toBe(0);
  });

  it('dispatches replay session updates directly like acp-ui', async () => {
    const { bridge, socket } = await connectBridge();
    const updates: unknown[] = [];
    bridge.onSessionUpdate = (notification) => updates.push(notification);
    const replayNotification = {
      jsonrpc: '2.0',
      method: 'session/update',
      params: {
        sessionId: 'session-1',
        update: {
          sessionUpdate: 'tool_call',
          toolCallId: 'tool-1',
          title: 'Read file',
          kind: 'read',
          status: 'completed',
          content: null,
          locations: null,
        },
      },
    };

    socket.triggerMessage(JSON.stringify(replayNotification));

    expect(updates).toEqual([replayNotification.params]);
    expect(socket.sent).toEqual([]);
    expect(console.error).not.toHaveBeenCalled();
  });

  it('keeps session/load pending when a replay update handler fails', async () => {
    const { bridge, socket } = await connectBridge();
    bridge.onSessionUpdate = () => {
      throw new Error('bad replay update');
    };

    const loadPromise = bridge.request('session/load', {
      sessionId: 'session-1',
      cwd: '/repo',
      mcpServers: [],
    });
    const frame = await readSentFrame(socket, 0);
    const replayNotification = {
      jsonrpc: '2.0',
      method: 'session/update',
      params: {
        sessionId: 'session-1',
        update: {
          sessionUpdate: 'available_commands_update',
          availableCommands: null,
        },
      },
    };

    socket.triggerMessage(JSON.stringify(replayNotification));
    respondWithResult(socket, frame, {});

    await expect(loadPromise).resolves.toEqual({});
    expect(console.error).toHaveBeenCalledWith(
      'Failed to handle ACP session/update notification:',
      expect.any(Error)
    );
  });

  it('keeps session/load pending when an inbound relay message is malformed', async () => {
    const { bridge, socket } = await connectBridge();

    const loadPromise = bridge.request('session/load', {
      sessionId: 'session-1',
      cwd: '/repo',
      mcpServers: [],
    });
    const frame = await readSentFrame(socket, 0);

    socket.triggerMessage('not-json');
    respondWithResult(socket, frame, {});

    await expect(loadPromise).resolves.toEqual({});
    expect(console.error).toHaveBeenCalledWith(
      'Failed to handle ACP relay message:',
      expect.any(Error)
    );
  });

  it('parses a single-message websocket frame', async () => {
    const { bridge, socket } = await connectBridge();
    const promise = bridge.request<{ ok: boolean }>('initialize', {});
    const frame = await readSentFrame(socket, 0);

    respondWithResult(socket, frame, { ok: true });

    await expect(promise).resolves.toEqual({ ok: true });
    expect(console.error).not.toHaveBeenCalled();
  });

  it('parses multiple newline-delimited websocket messages from one frame', async () => {
    const { bridge, socket } = await connectBridge();
    const first = bridge.request<{ first: boolean }>('initialize', {});
    const second = bridge.request<{ second: boolean }>('session/new', {});
    const firstFrame = await readSentFrame(socket, 0);
    const secondFrame = await readSentFrame(socket, 1);

    socket.triggerMessage([
      JSON.stringify({ jsonrpc: '2.0', id: firstFrame.id, result: { first: true } }),
      JSON.stringify({ jsonrpc: '2.0', id: secondFrame.id, result: { second: true } }),
    ].join('\n'));

    await expect(Promise.all([first, second])).resolves.toEqual([
      { first: true },
      { second: true },
    ]);
    expect(console.error).not.toHaveBeenCalled();
  });

  it('skips a trailing newline in a websocket frame', async () => {
    const { bridge, socket } = await connectBridge();
    const promise = bridge.request<{ ok: boolean }>('initialize', {});
    const frame = await readSentFrame(socket, 0);

    socket.triggerMessage(`${JSON.stringify({ jsonrpc: '2.0', id: frame.id, result: { ok: true } })}\n`);

    await expect(promise).resolves.toEqual({ ok: true });
    expect(console.error).not.toHaveBeenCalled();
  });

  it('skips mixed empty websocket frame lines while parsing non-empty lines', async () => {
    const { bridge, socket } = await connectBridge();
    const first = bridge.request<{ first: boolean }>('initialize', {});
    const second = bridge.request<{ second: boolean }>('session/new', {});
    const firstFrame = await readSentFrame(socket, 0);
    const secondFrame = await readSentFrame(socket, 1);

    socket.triggerMessage([
      '',
      '   ',
      JSON.stringify({ jsonrpc: '2.0', id: firstFrame.id, result: { first: true } }),
      '',
      JSON.stringify({ jsonrpc: '2.0', id: secondFrame.id, result: { second: true } }),
      ' ',
    ].join('\n'));

    await expect(Promise.all([first, second])).resolves.toEqual([
      { first: true },
      { second: true },
    ]);
    expect(console.error).not.toHaveBeenCalled();
  });

  it('surfaces malformed JSON per line while other websocket lines flow', async () => {
    const { bridge, socket } = await connectBridge();
    const first = bridge.request<{ first: boolean }>('initialize', {});
    const second = bridge.request<{ second: boolean }>('session/new', {});
    const firstFrame = await readSentFrame(socket, 0);
    const secondFrame = await readSentFrame(socket, 1);

    socket.triggerMessage([
      JSON.stringify({ jsonrpc: '2.0', id: firstFrame.id, result: { first: true } }),
      'not-json',
      JSON.stringify({ jsonrpc: '2.0', id: secondFrame.id, result: { second: true } }),
    ].join('\n'));

    await expect(Promise.all([first, second])).resolves.toEqual([
      { first: true },
      { second: true },
    ]);
    expect(console.error).toHaveBeenCalledWith(
      'Failed to handle ACP relay message:',
      expect.any(Error)
    );
  });

  it('uses long-lived timeouts for ACP control mutations', () => {
    const bridge = new TestAcpClientBridgeCore(
      'agent-1',
      'agent-1',
      new PermissionHandler(),
      createTestSinks()
    );

    expect(bridge.timeoutFor('session/set_config_option')).toBe(30 * 60 * 1000);
  });

  it('uses a 30 minute timeout for terminal/wait_for_exit', () => {
    const bridge = new TestAcpClientBridgeCore(
      'agent-1',
      'agent-1',
      new PermissionHandler(),
      createTestSinks()
    );

    expect(bridge.timeoutFor('terminal/wait_for_exit')).toBe(30 * 60 * 1000);
  });

  it('terminalCreate sends the ACP terminal/create request and returns a terminalId', async () => {
    const { bridge, socket } = await connectAcpBridge();
    const terminalCreate = (bridge as unknown as TerminalBridgeMethods).terminalCreate;

    expect(terminalCreate).toBeTypeOf('function');

    const promise = terminalCreate!.call(bridge, {
      sessionId: 'sess-1',
      command: 'npm',
      args: ['test'],
      env: [{ name: 'NODE_ENV', value: 'test' }],
      cwd: '/repo',
      outputByteLimit: 1024,
    });
    const frame = await readLatestSentFrame(socket);

    expect(frame).toMatchObject({
      method: 'terminal/create',
      params: {
        sessionId: 'sess-1',
        command: 'npm',
        args: ['test'],
        env: [{ name: 'NODE_ENV', value: 'test' }],
        cwd: '/repo',
        outputByteLimit: 1024,
      },
    });

    respondWithResult(socket, frame, { terminalId: 'term-1' });

    await expect(promise).resolves.toEqual({ terminalId: 'term-1' });
  });

  it('terminalOutput returns buffered output and exit status', async () => {
    const { bridge, socket } = await connectAcpBridge();
    const terminalOutput = (bridge as unknown as TerminalBridgeMethods).terminalOutput;

    expect(terminalOutput).toBeTypeOf('function');

    const promise = terminalOutput!.call(bridge, {
      sessionId: 'sess-1',
      terminalId: 'term-1',
    });
    const frame = await readLatestSentFrame(socket);

    expect(frame).toMatchObject({
      method: 'terminal/output',
      params: { sessionId: 'sess-1', terminalId: 'term-1' },
    });

    respondWithResult(socket, frame, {
      output: 'hello\n',
      truncated: false,
      exitStatus: { exitCode: 0, signal: null },
    });

    await expect(promise).resolves.toEqual({
      output: 'hello\n',
      truncated: false,
      exitStatus: { exitCode: 0, signal: null },
    });
  });

  it('terminalWaitForExit returns exitCode and signal', async () => {
    const { bridge, socket } = await connectAcpBridge();
    const terminalWaitForExit = (bridge as unknown as TerminalBridgeMethods).terminalWaitForExit;

    expect(terminalWaitForExit).toBeTypeOf('function');

    const promise = terminalWaitForExit!.call(bridge, {
      sessionId: 'sess-1',
      terminalId: 'term-1',
    });
    const frame = await readLatestSentFrame(socket);

    expect(frame).toMatchObject({
      method: 'terminal/wait_for_exit',
      params: { sessionId: 'sess-1', terminalId: 'term-1' },
    });

    respondWithResult(socket, frame, { exitCode: 0, signal: null });

    await expect(promise).resolves.toEqual({ exitCode: 0, signal: null });
  });

  it('terminalKill sends an ACP terminal/kill request', async () => {
    const { bridge, socket } = await connectAcpBridge();
    const terminalKill = (bridge as unknown as TerminalBridgeMethods).terminalKill;

    expect(terminalKill).toBeTypeOf('function');

    const promise = terminalKill!.call(bridge, {
      sessionId: 'sess-1',
      terminalId: 'term-1',
    });
    const frame = await readLatestSentFrame(socket);

    expect(frame).toMatchObject({
      method: 'terminal/kill',
      params: { sessionId: 'sess-1', terminalId: 'term-1' },
    });

    respondWithResult(socket, frame, {});

    await expect(promise).resolves.toEqual({});
  });

  it('terminalRelease acknowledges release and marks the store entry released', async () => {
    const { useTerminalsStore } = await loadUseTerminalsStoreModule();
    const store = useTerminalsStore();
    store.setTerminal('term-1', {
      terminalId: 'term-1',
      output: 'done',
      truncated: false,
      released: false,
      exitStatus: { exitCode: 0, signal: null },
    });
    const { bridge, socket } = await connectAcpBridge();
    const terminalRelease = (bridge as unknown as TerminalBridgeMethods).terminalRelease;

    expect(terminalRelease).toBeTypeOf('function');

    const promise = terminalRelease!.call(bridge, {
      sessionId: 'sess-1',
      terminalId: 'term-1',
    });
    const frame = await readLatestSentFrame(socket);

    expect(frame).toMatchObject({
      method: 'terminal/release',
      params: { sessionId: 'sess-1', terminalId: 'term-1' },
    });

    respondWithResult(socket, frame, {});

    await expect(promise).resolves.toEqual({});
    await nextTick();
    expect(store.getTerminalState('term-1').value?.released).toBe(true);
  });

  it('terminalOutput uses extMethod and arrives via SDK readable stream', async () => {
    const { bridge, socket } = await connectAcpBridge();
    const terminalOutput = (bridge as unknown as TerminalBridgeMethods).terminalOutput;

    expect(terminalOutput).toBeTypeOf('function');
    expect((bridge as unknown as { pendingManualRequests?: unknown }).pendingManualRequests).toBeUndefined();

    const promise = terminalOutput!.call(bridge, {
      sessionId: 'sess-1',
      terminalId: 'term-1',
    });
    const frame = await readLatestSentFrame(socket);

    expect(frame.method).toBe('terminal/output');
    expect((bridge as unknown as { pendingMethods: Map<unknown, string> }).pendingMethods.size).toBeGreaterThan(0);

    respondWithResult(socket, frame, {
      output: 'sdk-stream\n',
      truncated: false,
      exitStatus: { exitCode: 0, signal: null },
    });

    await expect(promise).resolves.toMatchObject({
      output: 'sdk-stream\n',
      truncated: false,
      exitStatus: { exitCode: 0, signal: null },
    });
  });

  it('terminal request races with session/new without desync', async () => {
    const { bridge, socket } = await connectAcpBridge();
    const terminalOutput = (bridge as unknown as TerminalBridgeMethods).terminalOutput;

    expect(terminalOutput).toBeTypeOf('function');

    const terminalPromise = terminalOutput!.call(bridge, {
      sessionId: 'sess-1',
      terminalId: 'term-1',
    });
    const sessionPromise = bridge.newSession({ cwd: '/repo', mcpServers: [] });

    await vi.waitFor(() => {
      expect(socket.sent.length).toBeGreaterThanOrEqual(2);
    });
    const frames = socket.sent.map((value) => JSON.parse(value) as JsonRpcTestFrame);
    const terminalFrame = frames.find((frame) => frame.method === 'terminal/output');
    const sessionFrame = frames.find((frame) => frame.method === 'session/new');

    expect(terminalFrame).toBeDefined();
    expect(sessionFrame).toBeDefined();

    respondWithResult(socket, terminalFrame!, {
      output: 'terminal first\n',
      truncated: false,
      exitStatus: { exitCode: 0, signal: null },
    });
    respondWithResult(socket, sessionFrame!, { sessionId: 'sess-new' });

    await expect(terminalPromise).resolves.toMatchObject({
      output: 'terminal first\n',
      truncated: false,
    });
    await expect(sessionPromise).resolves.toEqual({ sessionId: 'sess-new' });
  });

  it('removed manual-correlation methods are gone', async () => {
    const { bridge } = await connectAcpBridge();

    expect((bridge as unknown as { sendTerminalRequest?: unknown }).sendTerminalRequest).toBeUndefined();
    expect((bridge as unknown as { resolveManualResponse?: unknown }).resolveManualResponse).toBeUndefined();
    expect((bridge as unknown as { pendingManualRequests?: unknown }).pendingManualRequests).toBeUndefined();
  });

  it('terminalOutput surfaces unknown terminalId errors', async () => {
    const { bridge, socket } = await connectAcpBridge();
    const terminalOutput = (bridge as unknown as TerminalBridgeMethods).terminalOutput;

    expect(terminalOutput).toBeTypeOf('function');

    const promise = terminalOutput!.call(bridge, {
      sessionId: 'sess-1',
      terminalId: 'missing-terminal',
    });
    const frame = await readLatestSentFrame(socket);

    socket.triggerMessage(JSON.stringify({
      jsonrpc: '2.0',
      id: frame.id,
      error: { code: -32001, message: 'Unknown terminalId: missing-terminal' },
    }));

    await expect(promise).rejects.toThrow('Unknown terminalId: missing-terminal');
  });

  it('terminalCreate rejects when sessionId is missing', async () => {
    const { bridge } = await connectAcpBridge();
    const terminalCreate = (bridge as unknown as TerminalBridgeMethods).terminalCreate;

    expect(terminalCreate).toBeTypeOf('function');

    await expect(
      terminalCreate!.call(bridge, {
        sessionId: '',
        command: 'npm',
      })
    ).rejects.toThrow(/session/i);
  });

  it('terminalWaitForExit rejects malformed exit responses', async () => {
    const { bridge, socket } = await connectAcpBridge();
    const terminalWaitForExit = (bridge as unknown as TerminalBridgeMethods).terminalWaitForExit;

    expect(terminalWaitForExit).toBeTypeOf('function');

    const promise = terminalWaitForExit!.call(bridge, {
      sessionId: 'sess-1',
      terminalId: 'term-1',
    });
    const frame = await readLatestSentFrame(socket);

    socket.triggerMessage(JSON.stringify({
      jsonrpc: '2.0',
      id: frame.id,
      result: { exitCode: '0', signal: 9 },
    }));

    await expect(promise).rejects.toThrow(/exit/i);
  });
});
