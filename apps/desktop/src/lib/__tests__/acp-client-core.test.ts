import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { AcpClientBridgeCore } from '../acp-client-core';
import { PermissionHandler } from '../acp-permissions';
import { killAgent, onAgentClosed, onAgentMessage, sendToAgent } from '../tauri';
import { useTrafficStore } from '../../stores/traffic';

class TestAcpClientBridgeCore extends AcpClientBridgeCore {
  request<T>(method: string, params?: unknown): Promise<T> {
    return this.sendRequest<T>(method, params);
  }

  timeoutFor(method: string): number {
    return (this as any).getTimeoutForMethod(method);
  }

  receive(message: unknown): void {
    (this as any).handleMessage(JSON.stringify(message));
  }
}

describe('AcpClientBridgeCore request hardening', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    setActivePinia(createPinia());
    useTrafficStore().clear();
    vi.mocked(sendToAgent).mockReset().mockResolvedValue(undefined);
    vi.mocked(killAgent).mockReset().mockResolvedValue(undefined);
    vi.mocked(onAgentMessage).mockReset().mockResolvedValue(() => {});
    vi.mocked(onAgentClosed).mockReset().mockResolvedValue(() => {});
    vi.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  it('clears request timeout handles after a successful response', async () => {
    const bridge = new TestAcpClientBridgeCore('agent-1', new PermissionHandler());
    const promise = bridge.request<{ ok: boolean }>('initialize', {});

    expect(vi.getTimerCount()).toBe(1);

    bridge.receive({ jsonrpc: '2.0', id: 0, result: { ok: true } });

    await expect(promise).resolves.toEqual({ ok: true });
    expect(vi.getTimerCount()).toBe(0);
    expect(((bridge as any).timeoutHandles as Map<number, ReturnType<typeof setTimeout>>).size).toBe(0);
  });

  it('clears request timeout handles after an error response', async () => {
    const bridge = new TestAcpClientBridgeCore('agent-1', new PermissionHandler());
    const promise = bridge.request('initialize', {});

    expect(vi.getTimerCount()).toBe(1);

    bridge.receive({
      jsonrpc: '2.0',
      id: 0,
      error: { code: -32000, message: 'backend failed' },
    });

    await expect(promise).rejects.toThrow('backend failed');
    expect(vi.getTimerCount()).toBe(0);
    expect(((bridge as any).timeoutHandles as Map<number, ReturnType<typeof setTimeout>>).size).toBe(0);
  });

  it('clears request timeout handles when sendToAgent fails', async () => {
    vi.mocked(sendToAgent).mockRejectedValueOnce(new Error('send failed'));

    const bridge = new TestAcpClientBridgeCore('agent-1', new PermissionHandler());
    const promise = bridge.request('initialize', {});

    await expect(promise).rejects.toThrow('send failed');
    expect(vi.getTimerCount()).toBe(0);
    expect(((bridge as any).timeoutHandles as Map<number, ReturnType<typeof setTimeout>>).size).toBe(0);
  });

  it('rejects pending requests immediately on disconnect', async () => {
    vi.mocked(sendToAgent).mockImplementation(() => new Promise(() => {}));

    const bridge = new TestAcpClientBridgeCore('agent-1', new PermissionHandler());
    const promise = bridge.request('session/new', {});

    expect(vi.getTimerCount()).toBe(1);

    await bridge.disconnect();

    await expect(promise).rejects.toThrow('ACP client disconnected before response (agent-1)');
    expect(vi.mocked(killAgent)).toHaveBeenCalledWith('agent-1');
    expect(vi.getTimerCount()).toBe(0);
  });

  it('rejects pending requests when the agent terminates', async () => {
    let closedCallback: (agentId: string) => void = () => {};
    vi.mocked(onAgentClosed).mockImplementation(async (callback) => {
      closedCallback = callback;
      return () => {};
    });

    const bridge = new TestAcpClientBridgeCore('agent-1', new PermissionHandler());
    await bridge.connect();

    const promise = bridge.request('session/new', {});
    closedCallback('agent-1');

    await expect(promise).rejects.toThrow('ACP agent terminated before responding (agent-1)');
    expect(vi.getTimerCount()).toBe(0);
  });

  it('uses long-lived timeouts for ACP control mutations', () => {
    const bridge = new TestAcpClientBridgeCore('agent-1', new PermissionHandler());

    expect(bridge.timeoutFor('session/set_config_option')).toBe(30 * 60 * 1000);
  });
});
