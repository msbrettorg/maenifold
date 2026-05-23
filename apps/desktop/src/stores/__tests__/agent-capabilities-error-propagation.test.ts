import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { ref } from 'vue';

const connected = ref(false);
const reconnecting = ref(false);
const error = ref<string | null>(null);
const connect = vi.fn<() => Promise<void>>();
const disconnect = vi.fn();
const gatewayExecute = vi.fn();

vi.mock('../../composables/usePipelineMcp', () => ({
  usePipelineMcp: () => ({
    connected,
    reconnecting,
    error,
    connect,
    disconnect,
    gatewayExecute,
  }),
}));

async function createStore() {
  const notificationSink = await import('@/lib/notification-sink');
  const { useConfigStore } = await import('../config');
  const { useAgentCapabilitiesStore } = await import('../agent-capabilities');
  notificationSink.setNotificationSink(notificationSink.createNoopNotificationSink());
  const configStore = useConfigStore();
  configStore.config = {
    mcpServers: {},
    agents: {
      architect: { command: 'architect' },
    },
  };
  return useAgentCapabilitiesStore();
}

describe('agent capability probe error propagation', () => {
  beforeEach(() => {
    vi.resetModules();
    setActivePinia(createPinia());
    window.localStorage.clear();
    connected.value = false;
    reconnecting.value = false;
    error.value = null;
    connect.mockReset();
    disconnect.mockReset();
    gatewayExecute.mockReset();
  });

  afterEach(async () => {
    const notificationSink = await import('@/lib/notification-sink');
    notificationSink.resetNotificationSink();
  });

  it('surfaces the upstream connect rejection instead of the generic not-connected fallback', async () => {
    const store = await createStore();
    connect.mockRejectedValueOnce(new Error('UPSTREAM_CAUSE_X'));

    await store.probeAll({ force: true });

    expect(store.getProbe('architect')?.error ?? '').toContain('UPSTREAM_CAUSE_X');
  });

  it('surfaces Error subclass connect rejections', async () => {
    const store = await createStore();
    connect.mockRejectedValueOnce(new TypeError('port-in-use'));

    await store.probeAll({ force: true });

    expect(store.getProbe('architect')?.error ?? '').toContain('port-in-use');
  });

  it('surfaces non-Error connect rejections', async () => {
    const store = await createStore();
    connect.mockRejectedValueOnce('raw');

    await store.probeAll({ force: true });

    expect(store.getProbe('architect')?.error ?? '').toContain('raw');
  });

  it('sanitizes sensitive connect rejections before surfacing them', async () => {
    const store = await createStore();
    connect.mockRejectedValueOnce(new Error('Failed to load file:///Users/brett/secret/path/file.ts'));

    await store.probeAll({ force: true });

    const surfaced = store.getProbe('architect')?.error ?? '';
    expect(surfaced).toContain('<redacted>');
    expect(surfaced).not.toContain('file:///Users/brett/secret/path/file.ts');
  });

  it('keeps the probe error empty when connect succeeds', async () => {
    const store = await createStore();
    connect.mockResolvedValueOnce();
    connected.value = true;
    error.value = null;
    gatewayExecute.mockResolvedValue({
      agents: {
        architect: {
          command: 'architect',
        },
      },
      mcpServers: {},
      paths: {},
      settings: {},
    });

    await store.probeAll({ force: true });

    expect(store.getProbe('architect')?.error).toBeUndefined();
    expect(store.getProbe('architect')?.state).toBe('ready');
  });
});
