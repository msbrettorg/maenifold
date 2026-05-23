import { readdirSync, readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const COMPOSABLES_DIR = 'src/composables';
const HANDLER_PATH = `${COMPOSABLES_DIR}/useMessageHandler.ts`;
const EXPECTED_KEYS = [
  'cancelOperation',
  'cancelPendingToolCalls',
  'handleSessionUpdate',
  'pushSystemMessage',
  'sendPrompt',
] as const;

function lineCount(path: string): number {
  const source = readFileSync(path, 'utf8');
  return source === '' ? 0 : source.replace(/\r?\n$/, '').split('\n').length;
}

function extractReturnKeys(source: string): string[] {
  const matches = [...source.matchAll(/return\s*\{([\s\S]*?)\n\s*\};/g)];
  const match = matches.at(-1);
  if (!match) throw new Error('Could not find useMessageHandler() return block');

  return match[1]
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => line.replace(/,$/, ''))
    .map((line) => line.split(':')[0]?.trim() ?? '')
    .filter(Boolean)
    .sort();
}

function handlerFiles(): string[] {
  return readdirSync(COMPOSABLES_DIR)
    .filter((name) => /^useMessageHandler.*\.ts$/.test(name))
    .sort()
    .map((name) => `${COMPOSABLES_DIR}/${name}`);
}

describe('useMessageHandler split contract', () => {
  it('useMessageHandler.ts is ≤300 LOC', () => {
    expect(lineCount(HANDLER_PATH)).toBeLessThanOrEqual(300);
  });

  it('useMessageHandler() returns the locked public shape', () => {
    const keys = extractReturnKeys(readFileSync(HANDLER_PATH, 'utf8'));
    expect(keys).toEqual([...EXPECTED_KEYS]);
  });

  it('every .ts file in composables/ matching useMessageHandler*.ts is ≤300 LOC', () => {
    const files = handlerFiles();
    expect(files.length).toBeGreaterThan(0);
    for (const file of files) {
      expect(lineCount(file)).toBeLessThanOrEqual(300);
    }
  });
});
