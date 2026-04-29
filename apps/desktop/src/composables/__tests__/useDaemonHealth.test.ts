// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useDaemonHealth } from '../useDaemonHealth';

const fetchMock = vi.fn();

function okResponse(): Response {
  return { ok: true, status: 200, statusText: 'OK' } as Response;
}

async function flushPromises(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
}

describe('useDaemonHealth', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    fetchMock.mockReset();
    vi.stubGlobal('fetch', fetchMock);
  });

  afterEach(async () => {
    useDaemonHealth().stop();
    vi.unstubAllGlobals();
    vi.useRealTimers();
  });

  it('flips daemonOnline true to false on probe failure', async () => {
    const health = useDaemonHealth();
    fetchMock.mockResolvedValueOnce(okResponse()).mockRejectedValueOnce(new Error('offline'));

    await health.forceCheck();
    expect(health.daemonOnline.value).toBe(true);

    await health.forceCheck();

    expect(health.daemonOnline.value).toBe(false);
    expect(health.lastErrorMessage.value).toBe('offline');
  });

  it('flips daemonOnline false to true on probe success', async () => {
    const health = useDaemonHealth();
    fetchMock.mockRejectedValueOnce(new Error('offline')).mockResolvedValueOnce(okResponse());

    await health.forceCheck();
    expect(health.daemonOnline.value).toBe(false);

    await health.forceCheck();

    expect(health.daemonOnline.value).toBe(true);
    expect(health.lastErrorMessage.value).toBeNull();
  });

  it('forceCheck triggers an immediate probe', async () => {
    const health = useDaemonHealth();
    fetchMock.mockResolvedValueOnce(okResponse());

    await health.forceCheck();

    expect(fetchMock).toHaveBeenCalledOnce();
    expect(fetchMock.mock.calls[0]?.[0]).toContain('/api/health');
  });

  it('does not write probe state after stop aborts an in-flight check', async () => {
    const health = useDaemonHealth();
    const previousLastCheckedAt = health.lastCheckedAt.value;
    fetchMock.mockImplementationOnce((_url, init?: RequestInit) => new Promise((_resolve, reject) => {
      init?.signal?.addEventListener('abort', () => {
        reject(new DOMException('Aborted', 'AbortError'));
      });
    }));

    health.start();
    await flushPromises();
    expect(fetchMock).toHaveBeenCalledOnce();

    health.stop();
    await flushPromises();

    expect(health.daemonOnline.value).toBe(true);
    expect(health.lastErrorMessage.value).toBeNull();
    expect(health.lastCheckedAt.value).toBe(previousLastCheckedAt);
  });

  it('polls every 5s online and every 2s offline', async () => {
    const health = useDaemonHealth();
    fetchMock
      .mockResolvedValueOnce(okResponse())
      .mockResolvedValueOnce(okResponse())
      .mockRejectedValueOnce(new Error('offline'))
      .mockRejectedValueOnce(new Error('still offline'));

    health.start();
    await flushPromises();
    expect(fetchMock).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(4999);
    expect(fetchMock).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(1);
    await flushPromises();
    expect(fetchMock).toHaveBeenCalledTimes(2);

    await health.forceCheck();
    expect(health.daemonOnline.value).toBe(false);
    expect(fetchMock).toHaveBeenCalledTimes(3);

    await vi.advanceTimersByTimeAsync(1999);
    expect(fetchMock).toHaveBeenCalledTimes(3);
    await vi.advanceTimersByTimeAsync(1);
    await flushPromises();
    expect(fetchMock).toHaveBeenCalledTimes(4);
  });
});
