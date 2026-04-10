import { defineStore } from 'pinia';
import { ref, watch } from 'vue';
import { usePipelineMcp } from '../composables/usePipelineMcp';
import { unwrapToolData } from '../composables/pipeline-mcp-transport';
import type { AgentProbeResult } from '../lib/agent-probe-types';
import { useConfigStore } from './config';

type ProbeStatus = 'idle' | 'probing' | 'done' | 'error';

interface ProbeAgentRecord {
  name?: string;
  agent?: string;
  id?: string;
  error?: unknown;
  modes?: unknown;
  models?: unknown;
  configOptions?: unknown;
  capabilities?: {
    error?: unknown;
    modes?: unknown;
    models?: unknown;
    configOptions?: unknown;
  };
}

const RETRY_DELAY_MS = 3_000;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function normalizeStringList(value: unknown): string[] {
  return Array.isArray(value)
    ? value.filter((entry): entry is string => typeof entry === 'string')
    : [];
}

function normalizeConfigOptions(value: unknown): Record<string, unknown> {
  return isRecord(value) ? value : {};
}

function toProbeResult(record?: ProbeAgentRecord, error?: string): AgentProbeResult {
  const source = isRecord(record?.capabilities) ? record.capabilities : record;
  const embeddedError = typeof source?.error === 'string' ? source.error : undefined;
  return {
    modes: embeddedError ? [] : normalizeStringList(source?.modes),
    models: embeddedError ? [] : normalizeStringList(source?.models),
    configOptions: embeddedError ? {} : normalizeConfigOptions(source?.configOptions),
    probedAt: Date.now(),
    ...((error ?? embeddedError) ? { error: error ?? embeddedError } : {}),
  };
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
  const normalized = error.toLowerCase();
  return normalized.includes('not connected')
    || normalized.includes('network error')
    || normalized.includes('fetch')
    || normalized.includes('timed out')
    || normalized.includes('http 404');
}

export const useAgentCapabilitiesStore = defineStore('agent-capabilities', () => {
  const configStore = useConfigStore();
  const mcp = usePipelineMcp();
  const probeResults = ref<Map<string, AgentProbeResult>>(new Map());
  const probeStatus = ref<ProbeStatus>('idle');
  let currentRunId = 0;
  let retryHandle: ReturnType<typeof setTimeout> | null = null;
  let retryPending = false;

  function getProbe(agentName: string): AgentProbeResult | undefined {
    return probeResults.value.get(agentName);
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

  async function probeAll() {
    const runId = ++currentRunId;
    retryPending = false;
    clearRetry();

    const agentNames = [...configStore.agentNames];
    if (agentNames.length === 0) {
      probeResults.value = new Map();
      probeStatus.value = 'done';
      return;
    }

    probeStatus.value = 'probing';

    if (!(await ensureConnected())) {
      const message = mcp.error.value ?? 'MCP client is not connected';
      const next = new Map<string, AgentProbeResult>();
      for (const agentName of agentNames) {
        next.set(agentName, toProbeResult(undefined, message));
      }
      if (runId !== currentRunId) {
        return;
      }
      probeResults.value = next;
      probeStatus.value = 'error';
      scheduleRetry();
      return;
    }

    let hasErrors = false;
    let needsRetry = false;
    const next = new Map<string, AgentProbeResult>();

    try {
      const result = await mcp.callTool<unknown>('ListAgents', { probe: true });
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
      hasErrors = true;
      needsRetry = shouldRetryProbe(message);
      for (const agentName of agentNames) {
        next.set(agentName, toProbeResult(undefined, message));
      }
    }

    if (runId !== currentRunId) {
      return;
    }

    probeResults.value = next;
    probeStatus.value = hasErrors ? 'error' : 'done';

    if (needsRetry) {
      scheduleRetry();
    }
  }

  function invalidateAll() {
    currentRunId += 1;
    retryPending = false;
    clearRetry();
    probeResults.value = new Map();
    probeStatus.value = 'idle';
    void probeAll();
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
    getProbe,
    probeAll,
    invalidateAll,
  };
});
