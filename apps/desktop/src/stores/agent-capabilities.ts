import { defineStore } from 'pinia';
import { ref, watch } from 'vue';
import { usePipelineMcp } from '../composables/usePipelineMcp';
import { unwrapToolData } from '../composables/pipeline-mcp-transport';
import {
  normalizeAgentProbeResult,
  isOfflineProbeError,
  type AgentProbeResult,
} from '../lib/agent-probe-types';
import { useConfigStore } from './config';
import { useToastStore } from './toast';

type ProbeStatus = 'idle' | 'probing' | 'done' | 'error';

interface ProbeAgentRecord {
  name?: string;
  agent?: string;
  id?: string;
  capabilities?: {
    error?: unknown;
    [key: string]: unknown;
  };
  [key: string]: unknown;
}

const RETRY_DELAY_MS = 3_000;
export const AGENT_PROBE_CACHE_TTL_MS = 5 * 60_000;
export const AGENT_PROBE_SOFT_TIMEOUT_MS = 60_000;
const AGENT_PROBE_TIMEOUT_MESSAGE =
  `ListAgents probe timed out after ${AGENT_PROBE_SOFT_TIMEOUT_MS / 1_000}s`;
const AGENT_PROBE_FALLBACK_WARNING =
  'Live agent capability probe timed out. Showing configured agents and any cached capabilities until refresh succeeds.';

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function toProbeResult(record?: ProbeAgentRecord, error?: string): AgentProbeResult {
  const source = isRecord(record?.capabilities) ? record.capabilities : record;
  return normalizeAgentProbeResult(source, { error, probedAt: Date.now() });
}

function extractAgentRecords(result: unknown): ProbeAgentRecord[] {
  const data = unwrapToolData<unknown>(result);

  if (Array.isArray(data)) {
    return data.filter(isRecord) as ProbeAgentRecord[];
  }

  if (!isRecord(data)) {
    return [];
  }

  const nestedArrays = ['agents', 'data', 'results', 'items']
    .map((key) => data[key])
    .find(Array.isArray);

  if (Array.isArray(nestedArrays)) {
    return nestedArrays.filter(isRecord) as ProbeAgentRecord[];
  }

  if (isRecord(data.agents)) {
    return Object.entries(data.agents).map(([name, value]) => ({
      name,
      ...(isRecord(value) ? value : {}),
    }));
  }

  if ('name' in data || 'agent' in data || 'id' in data) {
    return [data as ProbeAgentRecord];
  }

  return Object.entries(data)
    .filter(([, value]) => isRecord(value))
    .map(([name, value]) => ({ name, ...(value as Record<string, unknown>) }));
}

function matchesAgent(record: ProbeAgentRecord, agentName: string): boolean {
  return [record.name, record.agent, record.id].some((value) => value === agentName);
}

function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function shouldRetryProbe(error: string): boolean {
  return isOfflineProbeError(error);
}

function isProbeTimeout(error: string): boolean {
  return error.includes(AGENT_PROBE_TIMEOUT_MESSAGE);
}

function stripProbeUiState(probe: AgentProbeResult): AgentProbeResult {
  const { refreshing: _refreshing, degradedMessage: _degradedMessage, ...rest } = probe;
  return rest;
}

function formatProbeAge(probedAt: number, now = Date.now()): string {
  const elapsedMs = Math.max(0, now - probedAt);
  const elapsedSeconds = Math.round(elapsedMs / 1_000);
  if (elapsedSeconds < 60) {
    return `${elapsedSeconds}s`;
  }

  const elapsedMinutes = Math.round(elapsedSeconds / 60);
  if (elapsedMinutes < 60) {
    return `${elapsedMinutes}m`;
  }

  const elapsedHours = Math.round(elapsedMinutes / 60);
  return `${elapsedHours}h`;
}

function refreshingMessage(probedAt: number): string {
  return `Refreshing live capabilities. Cached options from ${formatProbeAge(probedAt)} ago stay available until the probe finishes.`;
}

function cachedFailureMessage(probedAt: number, error: string): string {
  const age = formatProbeAge(probedAt);
  if (error.toLowerCase().includes('timed out')) {
    return `Live refresh timed out. Showing cached capabilities from ${age} ago.`;
  }

  return `Live probe is unavailable right now. Showing cached capabilities from ${age} ago.`;
}

async function listAgentsWithSoftTimeout(
  callTool: ReturnType<typeof usePipelineMcp>['callTool'],
): Promise<unknown> {
  let timeoutHandle: ReturnType<typeof setTimeout> | null = null;

  try {
    return await Promise.race([
      callTool<unknown>('ListAgents', { probe: true }),
      new Promise<never>((_, reject) => {
        timeoutHandle = setTimeout(() => {
          reject(new Error(AGENT_PROBE_TIMEOUT_MESSAGE));
        }, AGENT_PROBE_SOFT_TIMEOUT_MS);
      }),
    ]);
  } finally {
    if (timeoutHandle) {
      clearTimeout(timeoutHandle);
    }
  }
}

export const useAgentCapabilitiesStore = defineStore('agent-capabilities', () => {
  const configStore = useConfigStore();
  const mcp = usePipelineMcp();
  const toastStore = useToastStore();
  const probeResults = ref<Map<string, AgentProbeResult>>(new Map());
  const probeStatus = ref<ProbeStatus>('idle');
  const warningMessage = ref<string | null>(null);
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
    if (mcp.connected.value) {
      return true;
    }

    try {
      await mcp.connect();
    } catch {
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
      const message = mcp.error.value ?? 'MCP client is not connected';
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
      toastStore.addToast({ message, level: 'warning' });
      scheduleRetry();
      return;
    }

    let hasErrors = false;
    let needsRetry = false;
    const next = new Map<string, AgentProbeResult>();

    try {
      const result = await listAgentsWithSoftTimeout(mcp.callTool);
      const records = extractAgentRecords(result);

      for (const agentName of agentNames) {
        const match = records.find((record) => matchesAgent(record, agentName));
        if (!match) {
          hasErrors = true;
          next.set(agentName, toProbeResult(undefined, `Agent '${agentName}' not found in ListAgents response`));
          continue;
        }

        next.set(agentName, toProbeResult(match));
      }
    } catch (error) {
      const message = getErrorMessage(error);
      const timedOut = isProbeTimeout(message);
      hasErrors = true;
      needsRetry = shouldRetryProbe(message);
      const preserveCached = needsRetry;
      warningMessage.value = timedOut ? AGENT_PROBE_FALLBACK_WARNING : null;
      toastStore.addToast({
        message: warningMessage.value ?? message,
        level: 'warning',
      });
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
