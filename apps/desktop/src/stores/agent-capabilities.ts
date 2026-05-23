import { getNotificationSink } from '@/lib/notification-sink';
import { defineStore } from 'pinia';
import { ref, watch } from 'vue';
import { usePipelineMcp } from '../composables/usePipelineMcp';
import { type AgentProbeResult } from '../lib/agent-probe-types-defs';
import {
  loadAgentProbeCache,
  persistAgentProbeCache,
} from '../lib/agent-probe-cache';
import { sanitizeResourceErrorMessage, warnRawResourceErrorInDev } from '../lib/error-sanitizer';
import { useConfigStore } from './config';
import {
  AGENT_PROBE_CACHE_TTL_MS,
  AGENT_PROBE_FALLBACK_WARNING,
  cachedFailureMessage,
  extractAgentRecords,
  getConfigWithSoftTimeout,
  getErrorMessage,
  isProbeTimeout,
  matchesAgent,
  refreshingMessage,
  shouldRetryProbe,
  stripProbeUiState,
  toProbeResult,
} from './agent-capabilities-helpers';

export {
  AGENT_PROBE_CACHE_TTL_MS,
  AGENT_PROBE_SOFT_TIMEOUT_MS,
} from './agent-capabilities-helpers';

type ProbeStatus = 'idle' | 'probing' | 'done' | 'error';

const RETRY_DELAY_MS = 3_000;

export const useAgentCapabilitiesStore = defineStore('agent-capabilities', () => {
  const configStore = useConfigStore();
  const mcp = usePipelineMcp();
  const probeResults = ref<Map<string, AgentProbeResult>>(loadAgentProbeCache());
  const probeStatus = ref<ProbeStatus>('idle');
  const warningMessage = ref<string | null>(null);
  const connectErrorMessage = ref<string | null>(null);
  let currentRunId = 0;
  let retryHandle: ReturnType<typeof setTimeout> | null = null;
  let retryPending = false;

  function getProbe(agentName: string): AgentProbeResult | undefined {
    return probeResults.value.get(agentName);
  }

  function shouldProbeAgent(agentName: string, force = false): boolean {
    if (force) {
      return true;
    }

    const probe = probeResults.value.get(agentName);
    if (!probe) {
      return true;
    }

    if (probe.state !== 'ready' || probe.degradedMessage) {
      return true;
    }

    return Date.now() - probe.probedAt >= AGENT_PROBE_CACHE_TTL_MS;
  }

  function filterProbeResults(agentNames: string[]) {
    const allowed = new Set(agentNames);
    probeResults.value = new Map(
      [...probeResults.value.entries()].filter(([agentName]) => allowed.has(agentName)),
    );
  }

  function markRefreshPending(agentNames: string[]) {
    let changed = false;
    const next = new Map(probeResults.value);

    for (const agentName of agentNames) {
      const existing = next.get(agentName);
      if (!existing) {
        continue;
      }

      changed = true;
      next.set(agentName, {
        ...stripProbeUiState(existing),
        refreshing: true,
        degradedMessage:
          existing.state === 'ready'
            ? refreshingMessage(existing.probedAt)
            : 'Retrying the live capability probe…',
      });
    }

    if (changed) {
      probeResults.value = next;
    }
  }

  function clearRetry() {
    if (retryHandle) {
      clearTimeout(retryHandle);
      retryHandle = null;
    }
  }

  function scheduleRetry() {
    if (retryHandle) {
      return;
    }

    retryPending = true;
    retryHandle = setTimeout(() => {
      retryHandle = null;
      void probeAll();
    }, RETRY_DELAY_MS);
  }

  async function ensureConnected(): Promise<boolean> {
    connectErrorMessage.value = null;
    if (mcp.connected.value) {
      return true;
    }

    try {
      await mcp.connect();
    } catch (error) {
      const raw = error instanceof Error ? error.message : String(error);
      connectErrorMessage.value = sanitizeResourceErrorMessage(raw);
      return false;
    }

    return mcp.connected.value;
  }

  async function probeAll(options: { force?: boolean } = {}) {
    const runId = ++currentRunId;
    retryPending = false;
    clearRetry();

    const agentNames = [...configStore.agentNames];
    if (agentNames.length === 0) {
      probeResults.value = new Map();
      probeStatus.value = 'done';
      warningMessage.value = null;
      return;
    }

    filterProbeResults(agentNames);
    warningMessage.value = null;

    if (!agentNames.some((agentName) => shouldProbeAgent(agentName, options.force))) {
      probeStatus.value = 'done';
      return;
    }

    probeStatus.value = 'probing';
    markRefreshPending(agentNames);

    if (!(await ensureConnected())) {
      const message = connectErrorMessage.value ?? mcp.error.value ?? 'MCP client is not connected';
      const next = new Map<string, AgentProbeResult>();
      const preserveCached = shouldRetryProbe(message);
      for (const agentName of agentNames) {
        const existing = probeResults.value.get(agentName);
        if (preserveCached && existing?.state === 'ready') {
          next.set(agentName, {
            ...stripProbeUiState(existing),
            degradedMessage: cachedFailureMessage(existing.probedAt, message),
          });
          continue;
        }

        next.set(agentName, toProbeResult(undefined, message));
      }
      if (runId !== currentRunId) {
        return;
      }
      probeResults.value = next;
      probeStatus.value = 'error';
      getNotificationSink().warning(message);
      scheduleRetry();
      return;
    }

    let hasErrors = false;
    let needsRetry = false;
    const next = new Map<string, AgentProbeResult>();

    try {
      const result = await getConfigWithSoftTimeout(mcp.gatewayExecute);
      const records = extractAgentRecords(result);

      for (const agentName of agentNames) {
        const match = records.find((record) => matchesAgent(record, agentName));
        if (!match) {
          hasErrors = true;
          next.set(agentName, toProbeResult(undefined, `Agent '${agentName}' not found in GetConfig response`));
          continue;
        }

        next.set(agentName, toProbeResult(match));
      }
    } catch (error) {
      const rawError = getErrorMessage(error);
      warnRawResourceErrorInDev(rawError);
      const message = sanitizeResourceErrorMessage(rawError);
      const timedOut = isProbeTimeout(message);
      hasErrors = true;
      needsRetry = shouldRetryProbe(message);
      const preserveCached = needsRetry;
      warningMessage.value = timedOut ? AGENT_PROBE_FALLBACK_WARNING : null;
      getNotificationSink().warning(warningMessage.value ?? message);
      for (const agentName of agentNames) {
        const existing = probeResults.value.get(agentName);
        if (preserveCached && existing?.state === 'ready') {
          next.set(agentName, {
            ...stripProbeUiState(existing),
            degradedMessage: cachedFailureMessage(existing.probedAt, message),
          });
          continue;
        }

        if (timedOut) {
          continue;
        }

        next.set(agentName, toProbeResult(undefined, message));
      }
    }

    if (runId !== currentRunId) {
      return;
    }

    probeResults.value = next;
    probeStatus.value = hasErrors && !warningMessage.value ? 'error' : 'done';

    if (needsRetry) {
      scheduleRetry();
    }
  }

  function invalidateAll() {
    currentRunId += 1;
    retryPending = false;
    clearRetry();
    filterProbeResults([...configStore.agentNames]);
    warningMessage.value = null;
    probeStatus.value = probeResults.value.size > 0 ? 'done' : 'idle';
    void probeAll({ force: true });
  }

  async function refreshAll() {
    await probeAll({ force: true });
  }

  watch(mcp.connected, (connected) => {
    if (connected && retryPending) {
      retryPending = false;
      clearRetry();
      void probeAll();
    }
  });

  watch(probeResults, (next) => {
    persistAgentProbeCache(next);
  });

  return {
    probeResults,
    probeStatus,
    warningMessage,
    getProbe,
    probeAll,
    refreshAll,
    invalidateAll,
  };
});
