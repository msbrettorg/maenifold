import { computed, type ComputedRef } from 'vue';
import { useTerminalsStore, type TerminalState } from '../stores/useTerminalsStore';

function normalizeTerminalId(terminalId: string | undefined): string | null {
  if (typeof terminalId !== 'string') return null;
  const trimmed = terminalId.trim();
  return trimmed.length > 0 ? trimmed : null;
}

export function useTerminalOutput(terminalId: string | undefined): ComputedRef<TerminalState | null> {
  const store = useTerminalsStore();
  return computed(() => {
    const normalized = normalizeTerminalId(terminalId);
    return normalized ? store.terminals.get(normalized) ?? null : null;
  });
}
