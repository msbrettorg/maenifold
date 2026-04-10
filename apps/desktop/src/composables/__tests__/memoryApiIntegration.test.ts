// @vitest-environment jsdom
import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import type { MemoryFile, RecentResult, SearchResult } from '../../lib/workflow-types';
import { getApiBase, useMemoryApi } from '../useMemoryApi';

interface ProbeResult {
  available: boolean;
  reason?: string;
}

interface MutableTask {
  mode?: 'run' | 'skip' | 'only' | 'todo';
}

interface MutableSuite extends MutableTask {
  tasks?: MutableTask[];
}

const SERVER_ORIGIN = 'http://127.0.0.1:8080';
const EXPECTED_API_BASE = import.meta.env.VITE_MCP_BASE_URL ?? SERVER_ORIGIN;
const PROBE_TIMEOUT_MS = 3_000;
const INVALID_URI = 'memory://this/uri/should-not-exist';

let savedLocation: Location;
let discoveredMemoryUri: string | null = null;
let discoveredMemoryId: string | null = null;
let recentSnapshot: RecentResult | null = null;
let searchSnapshot: SearchResult | null = null;

function setWindowLocation(url: string): void {
  Object.defineProperty(window, 'location', {
    value: new URL(url),
    writable: true,
    configurable: true,
  });
}

function skipSuite(suite: MutableSuite): void {
  suite.mode = 'skip';
  suite.tasks?.forEach((task) => {
    task.mode = 'skip';
  });
}

async function fetchWithTimeout(input: RequestInfo | URL, init?: RequestInit): Promise<Response> {
  const controller = new AbortController();
  const timer = globalThis.setTimeout(() => controller.abort(), PROBE_TIMEOUT_MS);

  try {
    return await fetch(input, { ...init, signal: controller.signal });
  } finally {
    globalThis.clearTimeout(timer);
  }
}

async function probeMemoryApi(): Promise<ProbeResult> {
  try {
    const response = await fetchWithTimeout(
      `${SERVER_ORIGIN}/api/memory/recent?filter=thinking&limit=1`,
      { headers: { Accept: 'application/json' } },
    );

    if (!response.ok) {
      return {
        available: false,
        reason: `Probe returned HTTP ${response.status}`,
      };
    }

    return { available: true };
  } catch (error: unknown) {
    return {
      available: false,
      reason: error instanceof Error ? error.message : String(error),
    };
  }
}

function extractMemoryId(uri: string): string {
  return uri.slice(uri.lastIndexOf('/') + 1);
}

describe('getApiBase integration semantics', () => {
  beforeAll(() => {
    savedLocation = window.location;
  });

  afterAll(() => {
    Object.defineProperty(window, 'location', {
      value: savedLocation,
      writable: true,
      configurable: true,
    });
  });

  it('uses the configured backend for an http window origin', () => {
    setWindowLocation(`${SERVER_ORIGIN}/memory-viewer`);
    expect(getApiBase()).toBe(EXPECTED_API_BASE);
  });

  it('uses the configured backend for an https window origin', () => {
    setWindowLocation('https://example.test:9443/memory-viewer');
    expect(getApiBase()).toBe(EXPECTED_API_BASE);
  });

  it('uses the configured backend for non-http protocols', () => {
    setWindowLocation('file:///tmp/pipeline-ui/index.html');
    expect(getApiBase()).toBe(EXPECTED_API_BASE);
  });
});

describe('useMemoryApi live integration', () => {
  beforeAll(async ({}, suite) => {
    setWindowLocation(`${SERVER_ORIGIN}/memory-viewer`);
    const probe = await probeMemoryApi();

    if (!probe.available) {
      console.warn(
        `[memoryApiIntegration] Skipping live memory API tests: ${probe.reason ?? 'memory API is unavailable'}`,
      );
      skipSuite(suite as MutableSuite);
      return;
    }

    const api = useMemoryApi();
    recentSnapshot = await api.recentActivity({ filter: 'thinking', limit: 25 });
    discoveredMemoryUri = recentSnapshot.items[0]?.uri ?? null;

    if (!discoveredMemoryUri) {
      console.warn('[memoryApiIntegration] Skipping live memory API tests: no memories returned by /api/memory/recent.');
      skipSuite(suite as MutableSuite);
      return;
    }

    discoveredMemoryId = extractMemoryId(discoveredMemoryUri);
    searchSnapshot = await api.searchMemories(discoveredMemoryId, { pageSize: 25 });
  });

  beforeEach(() => {
    setWindowLocation(`${SERVER_ORIGIN}/memory-viewer`);
  });

  afterAll(() => {
    Object.defineProperty(window, 'location', {
      value: savedLocation,
      writable: true,
      configurable: true,
    });
  });

  it('readMemory fetches real content for a discovered memory URI', async () => {
    expect(discoveredMemoryUri).toEqual(expect.any(String));

    const { readMemory } = useMemoryApi();
    const memory = await readMemory(discoveredMemoryUri!);

    expect(memory).toMatchObject({
      uri: discoveredMemoryUri ?? undefined,
      title: expect.any(String),
      content: expect.any(String),
    } satisfies Partial<MemoryFile>);
    expect(memory.title.trim().length).toBeGreaterThan(0);
    expect(memory.content.trim().length).toBeGreaterThan(0);
  });

  it('searchMemories returns real results for a discovered query', async () => {
    expect(discoveredMemoryId).toEqual(expect.any(String));
    expect(searchSnapshot).not.toBeNull();

    expect(Array.isArray(searchSnapshot!.results)).toBe(true);
    expect(searchSnapshot!.results.length).toBeGreaterThan(0);
    expect(searchSnapshot!.results.some((item) => item.uri === discoveredMemoryUri)).toBe(true);
  });

  it('recentActivity returns real data from the live server', async () => {
    expect(recentSnapshot).not.toBeNull();

    expect(Array.isArray(recentSnapshot!.items)).toBe(true);
    expect(recentSnapshot!.items.length).toBeGreaterThan(0);
    expect(recentSnapshot!.items[0]).toMatchObject({
      uri: expect.any(String),
      title: expect.any(String),
      modified: expect.any(String),
      sections: expect.any(Array),
    });
  });

  it('surfaces a useful error for an invalid URI', async () => {
    const { readMemory } = useMemoryApi();

    await expect(readMemory(INVALID_URI)).rejects.toThrow(/404|not found|request failed/i);
  });
});
