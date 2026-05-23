import { describe, it, expect } from 'vitest';
import { readFileSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';

describe('session.ts split (T-336.6)', () => {
  it('stores/session.ts is at most 300 lines', () => {
    const src = readFileSync(
      resolve(__dirname, '../session.ts'),
      'utf8',
    );
    // wc -l semantics: count newlines, not visual lines
    const lineCount = (src.match(/\n/g) ?? []).length;
    expect(lineCount).toBeLessThanOrEqual(300);
  });

  it('composables/useSessionModels.ts exists', () => {
    const path = resolve(__dirname, '../../composables/useSessionModels.ts');
    expect(existsSync(path)).toBe(true);
  });

  it('composables/useSessionBridges.ts exists', () => {
    const path = resolve(__dirname, '../../composables/useSessionBridges.ts');
    expect(existsSync(path)).toBe(true);
  });
});
