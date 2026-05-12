import type { Ref } from 'vue';
import type { MemoryFile } from '../lib/workflow-types';

export interface KnowledgePollerDeps {
  listMemories: () => Promise<string>;
  readMemory: (uri: string) => Promise<MemoryFile>;
  activeTabUri: Ref<string | null>;
  intervalMs?: number;
}

export interface KnowledgePollerCallbacks {
  onTreeChanged: () => void;
  onActiveNoteChanged: (note: MemoryFile) => void;
}

export function createKnowledgePoller(
  deps: KnowledgePollerDeps,
  callbacks: KnowledgePollerCallbacks,
) {
  const { listMemories, readMemory, activeTabUri } = deps;
  const intervalMs = Math.max(2000, deps.intervalMs ?? 3000);

  let stopped = false;
  let running = false;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let lastListingFingerprint: string | null = null;
  let lastNoteFingerprint: string | null = null;
  let lastTrackedUri: string | null = null;

  function clearTimer(): void {
    if (timer !== null) {
      clearTimeout(timer);
      timer = null;
    }
  }

  function scheduleNext(): void {
    clearTimer();
    if (stopped) return;
    timer = setTimeout(() => { void runPoll(); }, intervalMs);
  }

  async function pollTree(): Promise<void> {
    try {
      const listing = await listMemories();
      if (lastListingFingerprint !== null && listing !== lastListingFingerprint) {
        callbacks.onTreeChanged();
      }
      lastListingFingerprint = listing;
    } catch {
      // tree poll failure is non-fatal; keep existing fingerprint
    }
  }

  async function pollActiveNote(): Promise<void> {
    const uri = activeTabUri.value;
    if (!uri) {
      lastTrackedUri = null;
      lastNoteFingerprint = null;
      return;
    }
    if (uri !== lastTrackedUri) {
      lastTrackedUri = uri;
      lastNoteFingerprint = null;
    }
    try {
      const note = await readMemory(uri);
      const fingerprint = note.checksum ?? note.modified ?? null;
      if (lastNoteFingerprint !== null && fingerprint !== lastNoteFingerprint) {
        callbacks.onActiveNoteChanged(note);
      }
      lastNoteFingerprint = fingerprint;
    } catch {
      // note poll failure is non-fatal; keep existing fingerprint
    }
  }

  async function runPoll(): Promise<void> {
    if (stopped) { running = false; return; }
    running = true;
    await pollTree();
    await pollActiveNote();
    running = false;
    scheduleNext();
  }

  function start(): void {
    if (running || (!stopped && timer !== null)) return;
    stopped = false;
    void runPoll();
  }

  function stop(): void {
    stopped = true;
    running = false;
    clearTimer();
  }

  return { start, stop };
}
