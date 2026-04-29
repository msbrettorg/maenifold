import { createPinia, setActivePinia } from 'pinia';
import { nextTick, type Ref } from 'vue';
import { beforeEach, describe, expect, it, vi } from 'vitest';

type TerminalState = {
  terminalId: string;
  output: string;
  truncated: boolean;
  released: boolean;
  exitStatus?: {
    exitCode?: number | null;
    signal?: string | null;
  } | null;
};

type UseTerminalsStore = {
  setTerminal: (terminalId: string, state: TerminalState) => void;
  appendOutput: (terminalId: string, chunk: string | null | undefined) => void;
  markReleased: (terminalId: string) => void;
};

type UseTerminalsStoreModule = {
  useTerminalsStore: () => UseTerminalsStore;
};

type UseTerminalOutputModule = {
  useTerminalOutput: (terminalId: string) => Ref<TerminalState | null>;
};

async function loadUseTerminalsStoreModule(): Promise<UseTerminalsStoreModule> {
  return vi.importActual<UseTerminalsStoreModule>('../useTerminalsStore');
}

async function loadUseTerminalOutputModule(): Promise<UseTerminalOutputModule> {
  return vi.importActual<UseTerminalOutputModule>('../useTerminalOutput');
}

describe('useTerminalOutput', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('returns a reactive ref to terminal state from the store', async () => {
    const [{ useTerminalOutput }, { useTerminalsStore }] = await Promise.all([
      loadUseTerminalOutputModule(),
      loadUseTerminalsStoreModule(),
    ]);
    const store = useTerminalsStore();
    store.setTerminal('term-1', {
      terminalId: 'term-1',
      output: 'hello',
      truncated: false,
      released: false,
      exitStatus: null,
    });

    const terminalState = useTerminalOutput('term-1');

    expect(terminalState.value?.output).toBe('hello');
  });

  it('updates when output is appended in the store', async () => {
    const [{ useTerminalOutput }, { useTerminalsStore }] = await Promise.all([
      loadUseTerminalOutputModule(),
      loadUseTerminalsStoreModule(),
    ]);
    const store = useTerminalsStore();
    store.setTerminal('term-1', {
      terminalId: 'term-1',
      output: 'hello',
      truncated: false,
      released: false,
      exitStatus: null,
    });

    const terminalState = useTerminalOutput('term-1');
    store.appendOutput('term-1', '\nworld');
    await nextTick();

    expect(terminalState.value?.output).toBe('hello\nworld');
  });

  it('returns null for an unknown terminalId', async () => {
    const { useTerminalOutput } = await loadUseTerminalOutputModule();

    const terminalState = useTerminalOutput('missing-terminal');

    expect(terminalState.value).toBeNull();
  });

  it('preserves the final state after terminal release', async () => {
    const [{ useTerminalOutput }, { useTerminalsStore }] = await Promise.all([
      loadUseTerminalOutputModule(),
      loadUseTerminalsStoreModule(),
    ]);
    const store = useTerminalsStore();
    store.setTerminal('term-1', {
      terminalId: 'term-1',
      output: 'final output',
      truncated: false,
      released: false,
      exitStatus: { exitCode: 0, signal: null },
    });

    const terminalState = useTerminalOutput('term-1');
    store.markReleased('term-1');
    await nextTick();

    expect(terminalState.value).toMatchObject({
      output: 'final output',
      released: true,
      exitStatus: { exitCode: 0, signal: null },
    });
  });
});
