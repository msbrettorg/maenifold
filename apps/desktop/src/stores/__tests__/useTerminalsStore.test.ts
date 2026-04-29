import { createPinia, setActivePinia } from 'pinia';
import { nextTick, type ComputedRef } from 'vue';
import { beforeEach, describe, expect, it, vi } from 'vitest';

type TerminalExitStatus = {
  exitCode?: number | null;
  signal?: string | null;
} | null;

type TerminalState = {
  terminalId: string;
  output: string;
  truncated: boolean;
  released: boolean;
  exitStatus?: TerminalExitStatus;
  outputByteLimit?: number;
};

type UseTerminalsStore = {
  terminals: Map<string, TerminalState>;
  setTerminal: (terminalId: string, state: TerminalState) => void;
  appendOutput: (terminalId: string, chunk: string | null | undefined) => void;
  markReleased: (terminalId: string) => void;
  markExited: (terminalId: string, exitStatus: Exclude<TerminalExitStatus, null>) => void;
  clear: (terminalId: string) => void;
  getTerminalState: (terminalId: string) => ComputedRef<TerminalState | null>;
};

type UseTerminalsStoreModule = {
  useTerminalsStore: () => UseTerminalsStore;
};

async function loadUseTerminalsStoreModule(): Promise<UseTerminalsStoreModule> {
  return vi.importActual<UseTerminalsStoreModule>('../useTerminalsStore');
}

function createTerminalState(overrides: Partial<TerminalState> = {}): TerminalState {
  return {
    terminalId: 'term-1',
    output: '',
    truncated: false,
    released: false,
    exitStatus: null,
    ...overrides,
  };
}

describe('useTerminalsStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('starts empty on init', async () => {
    const { useTerminalsStore } = await loadUseTerminalsStoreModule();
    const store = useTerminalsStore();

    expect(store.terminals.size).toBe(0);
  });

  it('setTerminal adds a terminal entry', async () => {
    const { useTerminalsStore } = await loadUseTerminalsStoreModule();
    const store = useTerminalsStore();

    store.setTerminal('term-1', createTerminalState({ output: 'hello' }));

    expect(store.terminals.get('term-1')).toMatchObject({
      terminalId: 'term-1',
      output: 'hello',
    });
  });

  it('appendOutput appends to an existing terminal', async () => {
    const { useTerminalsStore } = await loadUseTerminalsStoreModule();
    const store = useTerminalsStore();
    store.setTerminal('term-1', createTerminalState({ output: 'hello' }));

    store.appendOutput('term-1', '\nworld');

    expect(store.terminals.get('term-1')?.output).toBe('hello\nworld');
  });

  it('appendOutput creates an entry when terminal output arrives before terminal/create resolves', async () => {
    const { useTerminalsStore } = await loadUseTerminalsStoreModule();
    const store = useTerminalsStore();

    store.appendOutput('term-race', 'booting');

    expect(store.terminals.get('term-race')).toMatchObject({
      terminalId: 'term-race',
      output: 'booting',
      released: false,
    });
  });

  it('markReleased flips released to true but preserves the entry', async () => {
    const { useTerminalsStore } = await loadUseTerminalsStoreModule();
    const store = useTerminalsStore();
    store.setTerminal('term-1', createTerminalState({ output: 'done' }));

    store.markReleased('term-1');

    expect(store.terminals.get('term-1')).toMatchObject({
      output: 'done',
      released: true,
    });
  });

  it('markExited records exit info', async () => {
    const { useTerminalsStore } = await loadUseTerminalsStoreModule();
    const store = useTerminalsStore();
    store.setTerminal('term-1', createTerminalState());

    store.markExited('term-1', { exitCode: 0, signal: null });

    expect(store.terminals.get('term-1')?.exitStatus).toEqual({
      exitCode: 0,
      signal: null,
    });
  });

  it('clear removes the entry for session teardown', async () => {
    const { useTerminalsStore } = await loadUseTerminalsStoreModule();
    const store = useTerminalsStore();
    store.setTerminal('term-1', createTerminalState());

    store.clear('term-1');

    expect(store.terminals.has('term-1')).toBe(false);
  });

  it('retains output exactly at the byte limit without truncation', async () => {
    const { useTerminalsStore } = await loadUseTerminalsStoreModule();
    const store = useTerminalsStore();
    store.setTerminal('term-1', createTerminalState({ outputByteLimit: 4 }));

    store.appendOutput('term-1', 'abcd');

    expect(store.terminals.get('term-1')).toMatchObject({
      output: 'abcd',
      truncated: false,
    });
  });

  it('drops earliest bytes once output exceeds the byte limit', async () => {
    const { useTerminalsStore } = await loadUseTerminalsStoreModule();
    const store = useTerminalsStore();
    store.setTerminal('term-1', createTerminalState({ outputByteLimit: 4 }));

    store.appendOutput('term-1', 'abcde');

    expect(store.terminals.get('term-1')).toMatchObject({
      output: 'bcde',
      truncated: true,
    });
  });

  it('handles output far over the byte limit by keeping only the newest bytes', async () => {
    const { useTerminalsStore } = await loadUseTerminalsStoreModule();
    const store = useTerminalsStore();
    store.setTerminal('term-1', createTerminalState({ outputByteLimit: 4 }));

    store.appendOutput('term-1', 'abcdefghi');

    expect(store.terminals.get('term-1')).toMatchObject({
      output: 'fghi',
      truncated: true,
    });
  });

  it('does not split multi-byte UTF-8 characters during truncation', async () => {
    const { useTerminalsStore } = await loadUseTerminalsStoreModule();
    const store = useTerminalsStore();
    store.setTerminal('term-utf8', createTerminalState({ terminalId: 'term-utf8', outputByteLimit: 5 }));

    store.appendOutput('term-utf8', '🙂ab');

    expect(store.terminals.get('term-utf8')).toMatchObject({
      output: 'ab',
      truncated: true,
    });
  });

  it('getTerminalState returns a reactive getter', async () => {
    const { useTerminalsStore } = await loadUseTerminalsStoreModule();
    const store = useTerminalsStore();
    const terminalState = store.getTerminalState('term-1');

    expect(terminalState.value).toBeNull();

    store.setTerminal('term-1', createTerminalState({ output: 'hello' }));
    await nextTick();

    expect(terminalState.value?.output).toBe('hello');
  });

  it('ignores an empty terminalId', async () => {
    const { useTerminalsStore } = await loadUseTerminalsStoreModule();
    const store = useTerminalsStore();

    store.appendOutput('', 'ignored');

    expect(store.terminals.size).toBe(0);
  });

  it('ignores a whitespace-only terminalId', async () => {
    const { useTerminalsStore } = await loadUseTerminalsStoreModule();
    const store = useTerminalsStore();

    store.appendOutput('   ', 'ignored');

    expect(store.terminals.size).toBe(0);
  });

  it('ignores undefined output chunks', async () => {
    const { useTerminalsStore } = await loadUseTerminalsStoreModule();
    const store = useTerminalsStore();
    store.setTerminal('term-1', createTerminalState({ output: 'hello' }));

    store.appendOutput('term-1', undefined);

    expect(store.terminals.get('term-1')?.output).toBe('hello');
  });

  it('ignores null output chunks', async () => {
    const { useTerminalsStore } = await loadUseTerminalsStoreModule();
    const store = useTerminalsStore();
    store.setTerminal('term-1', createTerminalState({ output: 'hello' }));

    store.appendOutput('term-1', null);

    expect(store.terminals.get('term-1')?.output).toBe('hello');
  });

  it('handles a very large single-line chunk without newlines', async () => {
    const { useTerminalsStore } = await loadUseTerminalsStoreModule();
    const store = useTerminalsStore();
    const largeChunk = 'x'.repeat(1024 * 1024 + 128);
    store.setTerminal(
      'term-large',
      createTerminalState({
        terminalId: 'term-large',
        outputByteLimit: 1024,
      })
    );

    store.appendOutput('term-large', largeChunk);

    expect(store.terminals.get('term-large')).toMatchObject({
      output: 'x'.repeat(1024),
      truncated: true,
    });
  });
});
