import { defineStore } from 'pinia';
import { ref } from 'vue';
import { useMcpResources } from '../composables/useMcpResources';

const SAMPLING_SESSIONS_URI = 'sampling://sessions';
const MAX_HISTORY_ENTRIES = 50;

export interface SamplingHistoryTokenUsage {
  inputTokens: number;
  outputTokens: number;
}

export interface SamplingHistoryEntry {
  samplingRequestId: string;
  serverName: string;
  model: string;
  runtimeKind: string;
  roleName: string;
  status: string;
  decision: string;
  createdAt: string;
  tokenUsage: SamplingHistoryTokenUsage | null;
  acpSessionId?: string | null;
  appSessionId?: string;
}

interface SamplingSessionsPayload {
  sessions?: unknown;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function parseTokenUsage(value: unknown): SamplingHistoryTokenUsage | null | undefined {
  if (value === null) {
    return null;
  }
  if (!isRecord(value)) {
    return undefined;
  }
  if (typeof value.inputTokens !== 'number' || typeof value.outputTokens !== 'number') {
    return undefined;
  }
  return {
    inputTokens: value.inputTokens,
    outputTokens: value.outputTokens,
  };
}

function parseEntry(value: unknown): SamplingHistoryEntry | null {
  if (!isRecord(value)) {
    return null;
  }
  const tokenUsage = parseTokenUsage(value.tokenUsage);
  if (
    typeof value.samplingRequestId !== 'string'
    || typeof value.serverName !== 'string'
    || typeof value.model !== 'string'
    || typeof value.runtimeKind !== 'string'
    || typeof value.roleName !== 'string'
    || typeof value.status !== 'string'
    || typeof value.decision !== 'string'
    || typeof value.createdAt !== 'string'
    || tokenUsage === undefined
  ) {
    return null;
  }
  const entry: SamplingHistoryEntry = {
    samplingRequestId: value.samplingRequestId,
    serverName: value.serverName,
    model: value.model,
    runtimeKind: value.runtimeKind,
    roleName: value.roleName,
    status: value.status,
    decision: value.decision,
    createdAt: value.createdAt,
    tokenUsage,
  };
  if (typeof value.acpSessionId === 'string' || value.acpSessionId === null) {
    entry.acpSessionId = value.acpSessionId;
  }
  if (typeof value.appSessionId === 'string') {
    entry.appSessionId = value.appSessionId;
  }
  return entry;
}

function parseSessions(text: string | undefined): SamplingHistoryEntry[] {
  if (!text) {
    return [];
  }
  try {
    const payload = JSON.parse(text) as SamplingSessionsPayload;
    if (!Array.isArray(payload.sessions)) {
      return [];
    }
    return payload.sessions
      .map(parseEntry)
      .filter((entry): entry is SamplingHistoryEntry => entry !== null)
      .slice(-MAX_HISTORY_ENTRIES);
  } catch {
    return [];
  }
}

export const useSamplingHistoryStore = defineStore('sampling-history', () => {
  const entries = ref<SamplingHistoryEntry[]>([]);
  const resources = useMcpResources();

  async function refresh(): Promise<void> {
    const resource = await resources.readResource(SAMPLING_SESSIONS_URI);
    entries.value = parseSessions(resource.text);
  }

  async function init(): Promise<void> {
    await refresh();
  }

  return { entries, init, refresh };
});
