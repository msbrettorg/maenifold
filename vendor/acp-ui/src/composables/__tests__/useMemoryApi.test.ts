// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { MemoryFile } from '../../lib/workflow-types';
import { getApiBase, useMemoryApi } from '../useMemoryApi';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function setProtocol(protocol: string): void {
  Object.defineProperty(window, 'location', {
    value: { ...window.location, protocol },
    writable: true,
    configurable: true,
  });
}

// ---------------------------------------------------------------------------
// getApiBase — protocol detection
// ---------------------------------------------------------------------------

describe('getApiBase', () => {
  let savedLocation: Location;

  beforeEach(() => {
    savedLocation = window.location;
  });

  afterEach(() => {
    Object.defineProperty(window, 'location', {
      value: savedLocation,
      writable: true,
      configurable: true,
    });
    vi.unstubAllGlobals();
  });

  it('returns "" for http: protocol (--http and Tauri v2 frontendDist mode)', () => {
    setProtocol('http:');
    expect(getApiBase()).toBe('');
  });

  it('returns "" for https: protocol', () => {
    setProtocol('https:');
    expect(getApiBase()).toBe('');
  });

  it('returns explicit localhost base for tauri: protocol (legacy tauri://localhost)', () => {
    setProtocol('tauri:');
    expect(getApiBase()).toBe('http://localhost:8080');
  });

  it('returns explicit localhost base for any other exotic protocol', () => {
    setProtocol('file:');
    expect(getApiBase()).toBe('http://localhost:8080');
  });
});

// ---------------------------------------------------------------------------
// useMemoryApi — URL construction and fetch integration
// ---------------------------------------------------------------------------

describe('useMemoryApi', () => {
  const mockFile: MemoryFile = {
    uri: 'memory://path/to/file',
    title: 'Test File',
    content: '# Test\n\nContent here.',
    created: '2025-01-01T00:00:00Z',
    modified: '2025-01-01T00:00:00Z',
  };

  beforeEach(() => {
    // Ensure the SPA appears to be running on http: so relative URLs are used.
    setProtocol('http:');
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('readMemory calls fetch with a valid relative URL and decodes the response', async () => {
    const mockFetch = vi.fn().mockResolvedValue({
      ok: true,
      headers: { get: () => 'application/json' },
      json: () => Promise.resolve(mockFile),
    });
    vi.stubGlobal('fetch', mockFetch);

    const { readMemory } = useMemoryApi();
    const result = await readMemory('memory://path/to/file');

    expect(mockFetch).toHaveBeenCalledOnce();
    const calledUrl = mockFetch.mock.calls[0][0] as string;

    // Must be a valid URL (relative or absolute) — no TypeError from fetch.
    expect(() => new URL(calledUrl, 'http://localhost')).not.toThrow();

    // Must use a relative path (no hard-coded port override).
    expect(calledUrl).toMatch(/^\/api\/memory\/read\?/);
    expect(calledUrl).toContain('uri=memory%3A%2F%2Fpath%2Fto%2Ffile');

    expect(result).toEqual(mockFile);
  });

  it('searchMemories constructs a valid relative URL with query parameters', async () => {
    const mockFetch = vi.fn().mockResolvedValue({
      ok: true,
      headers: { get: () => 'application/json' },
      json: () =>
        Promise.resolve({ results: [], totalCount: 0, page: 1, pageSize: 10 }),
    });
    vi.stubGlobal('fetch', mockFetch);

    const { searchMemories } = useMemoryApi();
    await searchMemories('my query', { pageSize: 10, page: 2, folder: 'thinking' });

    const calledUrl = mockFetch.mock.calls[0][0] as string;
    expect(() => new URL(calledUrl, 'http://localhost')).not.toThrow();
    expect(calledUrl).toMatch(/^\/api\/memory\/search\?/);
    expect(calledUrl).toContain('query=my+query');
    expect(calledUrl).toContain('pageSize=10');
    expect(calledUrl).toContain('page=2');
    expect(calledUrl).toContain('folder=thinking');
  });

  it('recentActivity constructs a valid relative URL', async () => {
    const mockFetch = vi.fn().mockResolvedValue({
      ok: true,
      headers: { get: () => 'application/json' },
      json: () => Promise.resolve({ items: [] }),
    });
    vi.stubGlobal('fetch', mockFetch);

    const { recentActivity } = useMemoryApi();
    await recentActivity({ filter: 'thinking', limit: 50 });

    const calledUrl = mockFetch.mock.calls[0][0] as string;
    expect(() => new URL(calledUrl, 'http://localhost')).not.toThrow();
    expect(calledUrl).toMatch(/^\/api\/memory\/recent\?/);
    expect(calledUrl).toContain('filter=thinking');
    expect(calledUrl).toContain('limit=50');
  });

  it('throws when the server returns a non-OK response with a JSON error message', async () => {
    const mockFetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 404,
      headers: { get: () => 'application/json' },
      text: () => Promise.resolve(JSON.stringify({ message: 'Memory not found' })),
    });
    vi.stubGlobal('fetch', mockFetch);

    const { readMemory } = useMemoryApi();
    await expect(readMemory('memory://missing')).rejects.toThrow('Memory not found');
  });
});
