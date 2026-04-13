import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { ref } from 'vue';
import { persistAgentProbeCache } from '../../lib/agent-probe-cache';
import { normalizeAgentProbeResult } from '../../lib/agent-probe-types';
import { useConfigStore } from '../config';
import {
  AGENT_PROBE_CACHE_TTL_MS,
  AGENT_PROBE_SOFT_TIMEOUT_MS,
  useAgentCapabilitiesStore,
} from '../agent-capabilities';

const connected = ref(true);
const error = ref<string | null>(null);
const connect = vi.fn(async () => {
  connected.value = true;
});
const callTool = vi.fn();

vi.mock('../../composables/usePipelineMcp', () => ({
  usePipelineMcp: () => ({
    connected,
    error,
    connect,
    callTool,
  }),
}));

function seedConfig() {
  const configStore = useConfigStore();
  configStore.config = {
    mcpServers: [],
    agents: {
      architect: { command: 'architect' },
    },
  };
}

describe('useAgentCapabilitiesStore', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    vi.useRealTimers();
    setActivePinia(createPinia());
    window.localStorage.clear();
    connected.value = true;
    error.value = null;
    connect.mockClear();
    callTool.mockReset();
    seedConfig();
  });

  it('reuses fresh cached probe data until the TTL expires', async () => {
    const store = useAgentCapabilitiesStore();
    store.probeResults = new Map([
      ['architect', normalizeAgentProbeResult({ configOptions: [] }, { probedAt: Date.now() })],
    ]);

    await store.probeAll();

    expect(callTool).not.toHaveBeenCalled();
    expect(store.probeStatus).toBe('done');
  });

  it('forces a live refresh when requested even if the cache is still fresh', async () => {
    const store = useAgentCapabilitiesStore();
    store.probeResults = new Map([
      ['architect', normalizeAgentProbeResult({ configOptions: [] }, { probedAt: Date.now() })],
    ]);
    callTool.mockResolvedValue([
      { name: 'architect', capabilities: { configOptions: [] } },
    ]);

    await store.refreshAll();

    expect(callTool).toHaveBeenCalledWith('ListAgents', { probe: true });
    expect(store.getProbe('architect')?.state).toBe('ready');
    expect(store.getProbe('architect')?.degradedMessage).toBeUndefined();
  });

  it('hydrates persisted ready probes on store creation', () => {
    persistAgentProbeCache(new Map([
      ['architect', normalizeAgentProbeResult({
        configOptions: [
          {
            id: 'mode',
            name: 'Mode',
            category: 'mode',
            currentValue: 'prompt',
            options: [{ value: 'prompt', name: 'Prompt' }],
          },
        ],
      })],
    ]));

    const store = useAgentCapabilitiesStore();

    expect(store.getProbe('architect')?.state).toBe('ready');
    expect(store.getProbe('architect')?.modeOptions).toEqual([
      expect.objectContaining({ id: 'prompt', name: 'Prompt' }),
    ]);
  });

  it('keeps the last ready probe when a forced refresh times out', async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-04-11T12:00:00Z'));

    const store = useAgentCapabilitiesStore();
    const cachedProbedAt = Date.now() - AGENT_PROBE_CACHE_TTL_MS - 1_000;
    store.probeResults = new Map([
      ['architect', normalizeAgentProbeResult(
        {
          configOptions: [
            {
              id: 'mode',
              name: 'Mode',
              category: 'mode',
              currentValue: 'prompt',
              options: [{ value: 'prompt', name: 'Prompt' }],
            },
          ],
        },
        { probedAt: cachedProbedAt },
      )],
    ]);
    callTool.mockImplementation(() => new Promise(() => {}));

    const refreshPromise = store.refreshAll();

    expect(store.getProbe('architect')?.state).toBe('ready');
    expect(store.getProbe('architect')?.refreshing).toBe(true);
    expect(store.getProbe('architect')?.degradedMessage).toContain('Refreshing live capabilities');

    await vi.advanceTimersByTimeAsync(AGENT_PROBE_SOFT_TIMEOUT_MS);
    await refreshPromise;

    expect(store.getProbe('architect')?.state).toBe('ready');
    expect(store.getProbe('architect')?.refreshing).toBeUndefined();
    expect(store.getProbe('architect')?.degradedMessage).toContain('timed out');
    expect(store.getProbe('architect')?.probedAt).toBe(cachedProbedAt);
    expect(store.warningMessage).toContain('timed out');
  });

  it('falls back to the configured agent list when a probe times out without cache', async () => {
    vi.useFakeTimers();

    const store = useAgentCapabilitiesStore();
    callTool.mockImplementation(() => new Promise(() => {}));

    const refreshPromise = store.refreshAll();

    await vi.advanceTimersByTimeAsync(AGENT_PROBE_SOFT_TIMEOUT_MS);
    await refreshPromise;

    expect(store.getProbe('architect')).toBeUndefined();
    expect(store.probeStatus).toBe('done');
    expect(store.warningMessage).toContain('timed out');
  });

  it('does not surface legacy style from probe results', async () => {
    const store = useAgentCapabilitiesStore();
    callTool.mockResolvedValue([
      { name: 'architect', capabilities: { style: 'legacy' } },
    ]);

    await store.probeAll({ force: true });

    expect(store.getProbe('architect')?.style).toBe('unknown');
    expect(store.getProbe('architect')?.state).toBe('ready');
  });
});
