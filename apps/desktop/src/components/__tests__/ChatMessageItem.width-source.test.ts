import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

const sourcePath = join(process.cwd(), 'src/components/ChatMessageItem.vue');
const source = readFileSync(sourcePath, 'utf8');

function extractBracketedPercentCaps(text: string): number[] {
  return Array.from(text.matchAll(/max-w-\[(\d+)%\]/g), ([, value]) => Number(value));
}

describe('ChatMessageItem source width contract', () => {
  it('does not ship a bracketed max-width cap narrower than 95%', () => {
    expect(source).not.toContain('max-w-[80%]');
    expect(extractBracketedPercentCaps(source).every((value) => value >= 95)).toBe(true);
  });
});
