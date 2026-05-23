import { existsSync, readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const STARTUP_PATH = 'src/composables/useSessionStartup.ts';
const RESTORE_PATH = 'src/composables/useSessionRestore.ts';
const GREETING_PATH = 'src/composables/useSessionGreeting.ts';
const EXPECTED_KEYS = ['createSession', 'performInitialization', 'resumeSession'] as const;

function readLines(path: string): string[] {
  return readFileSync(path, 'utf8').split('\n');
}

function lineCount(path: string): number {
  return readLines(path).length;
}

function expectHelperWithinCap(path: string): void {
  if (!existsSync(path)) {
    throw new Error(`Missing helper composable: ${path}`);
  }

  expect(lineCount(path)).toBeLessThanOrEqual(300);
}

function extractReturnKeys(source: string): string[] {
  const matches = [...source.matchAll(/return\s*\{([\s\S]*?)\n\s*\};/g)];
  const match = matches.at(-1);
  if (!match) {
    throw new Error('Could not find useSessionStartup() return block');
  }

  return match[1]
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => line.replace(/,$/, ''))
    .map((line) => line.split(':')[0]?.trim() ?? '')
    .filter(Boolean)
    .sort();
}

describe('useSessionStartup split contract', () => {
  it('useSessionStartup.ts is ≤300 LOC', () => {
    expect(lineCount(STARTUP_PATH)).toBeLessThanOrEqual(300);
  });

  it('composables/ has useSessionRestore.ts helper', () => {
    expect(existsSync(RESTORE_PATH)).toBe(true);
  });

  it('composables/ has useSessionGreeting.ts helper', () => {
    expect(existsSync(GREETING_PATH)).toBe(true);
  });

  it('each new helper is ≤300 LOC', () => {
    expectHelperWithinCap(RESTORE_PATH);
    expectHelperWithinCap(GREETING_PATH);
  });

  it('useSessionStartup() returns the locked public shape', () => {
    // Static contract: there is no existing useSessionStartup test setup in this worktree,
    // so lock the public API by parsing the current source return block instead of adding mocks.
    const keys = extractReturnKeys(readFileSync(STARTUP_PATH, 'utf8'));

    expect(keys).toEqual([...EXPECTED_KEYS]);
  });
});
