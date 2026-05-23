import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

describe('acp-client-core decoupling (T-336.5)', () => {
  it('does not statically import any Pinia store from ../stores/', () => {
    const src = readFileSync(
      resolve(__dirname, '../acp-client-core.ts'),
      'utf8',
    );
    const storeImports = src.match(
      /from\s+['"]\.\.\/stores\/[^'"]+['"]/g,
    ) ?? [];
    expect(storeImports).toEqual([]);
  });

  it('does not reference useTrafficStore or useTerminalsStore symbols', () => {
    const src = readFileSync(
      resolve(__dirname, '../acp-client-core.ts'),
      'utf8',
    );
    expect(src).not.toMatch(/\buseTrafficStore\b/);
    expect(src).not.toMatch(/\buseTerminalsStore\b/);
  });
});
