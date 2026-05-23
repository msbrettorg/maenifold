import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const testDir = path.dirname(fileURLToPath(import.meta.url));
const composablesDir = path.resolve(testDir, '..');
const parentFile = path.join(composablesDir, 'useStreamHandler.ts');
const EXPECTED_KEYS = [
  'cloneContentBlock',
  'handleAgentMessageChunk',
  'handleAgentThoughtChunk',
  'handleToolCall',
  'handleToolCallUpdate',
  'handleUserMessageChunk',
].sort();

function countLines(text: string): number {
  return text === '' ? 0 : text.split('\n').length - Number(text.endsWith('\n'));
}

function extractLockedPublicKeys(source: string): string[] {
  // Static source parse: this file's current public contract is its named exports,
  // which consumers import directly. We avoid runtime setup for Vue/Pinia/ACP here.
  return [...source.matchAll(/export function (\w+)/g)].map(([, name]) => name).sort();
}

describe('useStreamHandler split contract', () => {
  it('useStreamHandler.ts is ≤300 LOC', () => {
    expect(countLines(fs.readFileSync(parentFile, 'utf8'))).toBeLessThanOrEqual(300);
  });

  it('useStreamHandler() returns the locked public shape', () => {
    expect(extractLockedPublicKeys(fs.readFileSync(parentFile, 'utf8'))).toEqual(EXPECTED_KEYS);
  });

  it('every .ts file in composables/ matching useStreamHandler*.ts is ≤300 LOC', () => {
    const oversized = fs
      .readdirSync(composablesDir)
      .filter((name) => /^useStreamHandler.*\.ts$/.test(name) && !name.includes('.test.'))
      .map((name) => ({
        name,
        lines: countLines(fs.readFileSync(path.join(composablesDir, name), 'utf8')),
      }))
      .filter(({ lines }) => lines > 300);

    expect(oversized).toEqual([]);
  });
});
