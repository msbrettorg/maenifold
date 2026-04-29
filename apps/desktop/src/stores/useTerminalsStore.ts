import { computed, reactive, type ComputedRef } from 'vue';
import { defineStore } from 'pinia';

export type TerminalExitStatus = {
  exitCode?: number | null;
  signal?: string | null;
} | null;

export type TerminalState = {
  terminalId: string;
  output: string;
  truncated: boolean;
  released: boolean;
  exitStatus?: TerminalExitStatus;
  outputByteLimit?: number;
};

const encoder = new TextEncoder();

function normalizeTerminalId(terminalId: string | null | undefined): string | null {
  if (typeof terminalId !== 'string') return null;
  const trimmed = terminalId.trim();
  return trimmed.length > 0 ? trimmed : null;
}

function validByteLimit(limit: number | null | undefined): number | null {
  return typeof limit === 'number' && Number.isFinite(limit) && limit >= 0
    ? Math.floor(limit)
    : null;
}

function truncateUtf8FromFront(output: string, limit: number): { output: string; truncated: boolean } {
  const bytes = encoder.encode(output);
  if (bytes.byteLength <= limit) {
    return { output, truncated: false };
  }
  if (limit === 0) {
    return { output: '', truncated: true };
  }

  const firstCandidate = Math.max(0, bytes.byteLength - limit);
  for (let start = firstCandidate; start < bytes.byteLength; start += 1) {
    try {
      return {
        output: new TextDecoder('utf-8', { fatal: true }).decode(bytes.slice(start)),
        truncated: true,
      };
    } catch {
      // Continue until the retained suffix starts at a valid UTF-8 boundary.
    }
  }

  return { output: '', truncated: true };
}

function createDefaultState(terminalId: string, outputByteLimit?: number): TerminalState {
  return {
    terminalId,
    output: '',
    truncated: false,
    released: false,
    exitStatus: null,
    ...(outputByteLimit == null ? {} : { outputByteLimit }),
  };
}

export const useTerminalsStore = defineStore('terminals', () => {
  const terminals = reactive(new Map<string, TerminalState>());

  function setTerminal(terminalId: string, state: TerminalState): void {
    const normalized = normalizeTerminalId(terminalId);
    if (!normalized) return;

    const existing = terminals.get(normalized);
    const outputByteLimit = validByteLimit(state.outputByteLimit);
    const base: TerminalState = {
      ...createDefaultState(normalized, outputByteLimit ?? undefined),
      ...existing,
      ...state,
      terminalId: normalized,
      outputByteLimit: outputByteLimit ?? existing?.outputByteLimit,
    };
    if (base.outputByteLimit == null) {
      delete base.outputByteLimit;
    }
    terminals.set(normalized, base);
  }

  function appendOutput(terminalId: string, chunk: string | null | undefined): void {
    const normalized = normalizeTerminalId(terminalId);
    if (!normalized || chunk == null || chunk.length === 0) return;

    const existing = terminals.get(normalized) ?? createDefaultState(normalized);
    const appended = `${existing.output}${chunk}`;
    const limit = validByteLimit(existing.outputByteLimit);
    const truncated = limit == null
      ? { output: appended, truncated: false }
      : truncateUtf8FromFront(appended, limit);

    terminals.set(normalized, {
      ...existing,
      output: truncated.output,
      truncated: existing.truncated || truncated.truncated,
    });
  }

  function markReleased(terminalId: string): void {
    const normalized = normalizeTerminalId(terminalId);
    if (!normalized) return;
    const existing = terminals.get(normalized) ?? createDefaultState(normalized);
    terminals.set(normalized, { ...existing, released: true });
  }

  function markExited(
    terminalId: string,
    exitStatus: Exclude<TerminalExitStatus, null>
  ): void {
    const normalized = normalizeTerminalId(terminalId);
    if (!normalized) return;
    const existing = terminals.get(normalized) ?? createDefaultState(normalized);
    terminals.set(normalized, { ...existing, exitStatus });
  }

  function clear(terminalId: string): void {
    const normalized = normalizeTerminalId(terminalId);
    if (!normalized) return;
    terminals.delete(normalized);
  }

  function getTerminalState(terminalId: string): ComputedRef<TerminalState | null> {
    return computed(() => {
      const normalized = normalizeTerminalId(terminalId);
      return normalized ? terminals.get(normalized) ?? null : null;
    });
  }

  const api = typeof window === 'undefined' ? undefined : window.electronAPI?.terminal;
  if (api?.onOutput) {
    api.onOutput((terminalId, chunk) => appendOutput(terminalId, chunk));
  }

  return {
    terminals,
    setTerminal,
    appendOutput,
    markReleased,
    markExited,
    clear,
    getTerminalState,
  };
});
