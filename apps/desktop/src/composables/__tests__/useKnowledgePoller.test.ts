import { flushPromises } from '@vue/test-utils';
import { ref } from 'vue';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createKnowledgePoller } from '../useKnowledgePoller';
import type { MemoryFile } from '../../lib/workflow-types';

function makeNote(overrides: Partial<MemoryFile> = {}): MemoryFile {
  return {
    uri: 'memory://test/note',
    title: 'Test Note',
    content: 'Hello.',
    checksum: 'cs1',
    created: null,
    modified: '2024-01-01T00:00:00Z',
    ...overrides,
  };
}

/** Advance the clock by one poll interval and flush all pending promises. */
async function tickPoll(ms = 2000): Promise<void> {
  await vi.advanceTimersByTimeAsync(ms);
  await flushPromises();
}

describe('createKnowledgePoller', () => {
  beforeEach(() => { vi.useFakeTimers(); });
  afterEach(() => { vi.useRealTimers(); });

  it('fires onTreeChanged when listing fingerprint changes after first poll', async () => {
    let callCount = 0;
    const listMemories = vi.fn(async () => {
      callCount += 1;
      return callCount === 1 ? 'listing-A' : 'listing-B';
    });
    const readMemory = vi.fn(async () => makeNote());
    const activeTabUri = ref<string | null>(null);
    const onTreeChanged = vi.fn();

    const poller = createKnowledgePoller(
      { listMemories, readMemory, activeTabUri, intervalMs: 2000 },
      { onTreeChanged, onActiveNoteChanged: vi.fn() },
    );

    poller.start();
    // First poll runs immediately on start — flush its async work
    await flushPromises();
    expect(onTreeChanged).not.toHaveBeenCalled();

    // Second poll triggered by the scheduled timer
    await tickPoll();
    expect(onTreeChanged).toHaveBeenCalledOnce();

    poller.stop();
  });

  it('fires onActiveNoteChanged when note fingerprint changes after first poll', async () => {
    let noteCallCount = 0;
    const readMemory = vi.fn(async () => {
      noteCallCount += 1;
      return makeNote({ checksum: noteCallCount === 1 ? 'cs1' : 'cs2' });
    });
    const listMemories = vi.fn(async () => 'same-listing');
    const activeTabUri = ref<string | null>('memory://test/note');
    const onActiveNoteChanged = vi.fn();

    const poller = createKnowledgePoller(
      { listMemories, readMemory, activeTabUri, intervalMs: 2000 },
      { onTreeChanged: vi.fn(), onActiveNoteChanged },
    );

    poller.start();
    await flushPromises();
    expect(onActiveNoteChanged).not.toHaveBeenCalled();

    await tickPoll();
    expect(onActiveNoteChanged).toHaveBeenCalledOnce();
    expect(onActiveNoteChanged.mock.calls[0][0].checksum).toBe('cs2');

    poller.stop();
  });

  it('does not poll after stop is called', async () => {
    const listMemories = vi.fn(async () => 'listing');
    const readMemory = vi.fn(async () => makeNote());
    const activeTabUri = ref<string | null>(null);

    const poller = createKnowledgePoller(
      { listMemories, readMemory, activeTabUri, intervalMs: 2000 },
      { onTreeChanged: vi.fn(), onActiveNoteChanged: vi.fn() },
    );

    poller.start();
    await flushPromises();
    const callsAfterFirstPoll = listMemories.mock.calls.length;

    poller.stop();
    await tickPoll();
    await tickPoll();
    expect(listMemories.mock.calls.length).toBe(callsAfterFirstPoll);
  });

  it('resets note fingerprint when activeTabUri changes', async () => {
    const activeTabUri = ref<string | null>('memory://test/a');
    let uriQueried: string[] = [];
    const readMemory = vi.fn(async (uri: string) => {
      uriQueried.push(uri);
      return makeNote({ uri, checksum: 'same' });
    });
    const onActiveNoteChanged = vi.fn();

    const poller = createKnowledgePoller(
      { listMemories: vi.fn(async () => 'L'), readMemory, activeTabUri, intervalMs: 2000 },
      { onTreeChanged: vi.fn(), onActiveNoteChanged },
    );

    poller.start();
    await flushPromises();

    // Switch to a different note — reset fingerprint
    activeTabUri.value = 'memory://test/b';
    uriQueried = [];

    // First poll for new URI sets baseline — no callback should fire
    await tickPoll();
    expect(uriQueried).toContain('memory://test/b');
    expect(onActiveNoteChanged).not.toHaveBeenCalled();

    poller.stop();
  });
});
