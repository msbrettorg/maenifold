import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it } from 'vitest';
import { useTerminalsStore } from '../useTerminalsStore';

const DEFAULT_OUTPUT_BYTE_LIMIT = 1_048_576;

describe('useTerminalsStore renderer output caps', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('truncates repeated live chunks using the normalized default outputByteLimit', () => {
    const store = useTerminalsStore();
    store.setTerminal('term-default', {
      terminalId: 'term-default',
      output: '',
      truncated: false,
      released: false,
      exitStatus: null,
      outputByteLimit: DEFAULT_OUTPUT_BYTE_LIMIT,
    });

    store.appendOutput('term-default', 'x'.repeat(400_000));
    store.appendOutput('term-default', 'x'.repeat(400_000));
    store.appendOutput('term-default', 'x'.repeat(400_000));

    const state = store.terminals.get('term-default');
    expect(state?.truncated).toBe(true);
    expect(state?.output.length).toBeLessThanOrEqual(DEFAULT_OUTPUT_BYTE_LIMIT);
  });

  it('preserves UTF-8 boundaries when the normalized default cap truncates live chunks', () => {
    const store = useTerminalsStore();
    store.setTerminal('term-utf8', {
      terminalId: 'term-utf8',
      output: '',
      truncated: false,
      released: false,
      exitStatus: null,
      outputByteLimit: 5,
    });

    store.appendOutput('term-utf8', '🙂ab');

    expect(store.terminals.get('term-utf8')).toMatchObject({
      output: 'ab',
      truncated: true,
    });
  });
});
