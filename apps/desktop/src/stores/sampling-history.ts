import { defineStore } from 'pinia';
import { ref } from 'vue';
import { useMcpResources } from '../composables/useMcpResources';
import { useMcpClient } from '../composables/useMcpClient';
import { getApiBase } from '../lib/api-base';
import type { SamplingHistoryEntry, SamplingHistoryTokenUsage } from '../lib/types';
export type { SamplingHistoryEntry, SamplingHistoryTokenUsage } from '../lib/types';

const SAMPLING_SESSIONS_URI = 'sampling://sessions';
const MAX_HISTORY_ENTRIES = 50;
const LIVE_STATUSES = new Set(['dispatching', 'streaming', 'budgetExceeded']);

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
  const statusInfo = parseStatus(value.status);
  if (
    typeof value.samplingRequestId !== 'string'
    || typeof value.serverName !== 'string'
    || typeof value.model !== 'string'
    || typeof value.runtimeKind !== 'string'
    || typeof value.roleName !== 'string'
    || statusInfo === null
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
    status: statusInfo.status,
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
  if (typeof value.toolCallCount === 'number') entry.toolCallCount = value.toolCallCount;
  if (typeof value.toolLoopBudget === 'number') entry.toolLoopBudget = value.toolLoopBudget;
  if (typeof value.turnCount === 'number') entry.turnCount = value.turnCount;
  if (statusInfo.allowedSoFar !== undefined) entry.toolCallCount = statusInfo.allowedSoFar;
  if (statusInfo.budget !== undefined) entry.toolLoopBudget = statusInfo.budget;
  return entry;
}

function parseStatus(value: unknown): { status: string; allowedSoFar?: number; budget?: number } | null {
  if (typeof value === 'string') return { status: value };
  if (!isRecord(value) || !isRecord(value.budgetExceeded)) return null;
  const payload = value.budgetExceeded;
  return {
    status: 'budgetExceeded',
    allowedSoFar: typeof payload.allowedSoFar === 'number' ? payload.allowedSoFar : undefined,
    budget: typeof payload.budget === 'number' ? payload.budget : undefined,
  };
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
  const mcp = useMcpClient();
  const eventSources = new Map<string, EventSource>();
  let unsubscribeResource: (() => void) | null = null;
  let started = false;

  function liveKey(entry: SamplingHistoryEntry): string | null {
    return entry.appSessionId ?? null;
  }

  function mergeCurrentDeltas(nextEntries: SamplingHistoryEntry[]): SamplingHistoryEntry[] {
    const currentByRequest = new Map(entries.value.map((entry) => [entry.samplingRequestId, entry.currentDelta]));
    return nextEntries.map((entry) => ({
      ...entry,
      currentDelta: currentByRequest.get(entry.samplingRequestId) ?? entry.currentDelta,
    }));
  }

  function applySsePayload(appSessionId: string, rawData: string): void {
    let payload: { type?: string; appSessionId?: string; delta?: string };
    try {
      payload = JSON.parse(rawData) as { type?: string; appSessionId?: string; delta?: string };
    } catch {
      return;
    }
    if (payload.appSessionId !== appSessionId || payload.type !== 'acpDelta' || typeof payload.delta !== 'string') {
      return;
    }
    entries.value = entries.value.map((entry) => {
      if (entry.appSessionId !== appSessionId) return entry;
      return { ...entry, currentDelta: `${entry.currentDelta ?? ''}${payload.delta}` };
    });
  }

  function syncEventSources(): void {
    const activeKeys = new Set<string>();
    for (const entry of entries.value) {
      const key = liveKey(entry);
      if (!key || !LIVE_STATUSES.has(entry.status)) continue;
      activeKeys.add(key);
      if (eventSources.has(key)) continue;
      const source = new EventSource(`${getApiBase()}/events/sampling/${encodeURIComponent(key)}`);
      source.onmessage = (event) => applySsePayload(key, event.data);
      source.onerror = () => {
        source.close();
        eventSources.delete(key);
      };
      eventSources.set(key, source);
    }
    for (const [key, source] of eventSources.entries()) {
      if (activeKeys.has(key)) continue;
      source.close();
      eventSources.delete(key);
    }
  }

  async function refresh(): Promise<void> {
    const resource = await resources.readResource(SAMPLING_SESSIONS_URI);
    entries.value = mergeCurrentDeltas(parseSessions(resource.text));
    syncEventSources();
  }

  async function init(): Promise<void> {
    await start();
  }

  async function start(): Promise<void> {
    if (started) return;
    started = true;
    await refresh();
    unsubscribeResource = await resources.subscribeResource(SAMPLING_SESSIONS_URI, () => {
      void refresh();
    }).catch(() => null);
  }

  async function stop(): Promise<void> {
    started = false;
    if (unsubscribeResource) {
      await unsubscribeResource();
      unsubscribeResource = null;
    }
    for (const source of eventSources.values()) {
      source.close();
    }
    eventSources.clear();
  }

  async function cancelSamplingSession(samplingRequestId: string): Promise<void> {
    entries.value = entries.value.map((entry) => (
      entry.samplingRequestId === samplingRequestId ? { ...entry, status: 'cancelled' } : entry
    ));
    await mcp.connect().catch(() => undefined);
    await mcp.callTool('CancelSamplingSession', { samplingRequestId });
    await refresh();
  }

  return { entries, init, start, stop, refresh, cancelSamplingSession };
});
