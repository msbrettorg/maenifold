import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

describe('CDP perf harness gating (T-336.3 regression pin)', () => {
  const mainTs = readFileSync(
    resolve(__dirname, '../main.ts'),
    'utf8',
  );
  const preloadTs = readFileSync(
    resolve(__dirname, '../../electron/preload.ts'),
    'utf8',
  );

  it('main.ts gates the CDP harness import on DEV OR UAT runtime flag', () => {
    // Must contain both clauses of the OR-gate.
    expect(mainTs).toMatch(/import\.meta\.env\.DEV/);
    expect(mainTs).toMatch(/window\.__MAENIFOLD_RUNTIME__/);
    expect(mainTs).toMatch(/await import\(['"]\.\/lib\/cdp-perf-harness['"]\)/);
  });

  it('main.ts does NOT static-import the cdp-perf-harness module', () => {
    // Static import would defeat tree-shaking.
    expect(mainTs).not.toMatch(
      /^import\s+[^;]*from\s+['"][^'"]*cdp-perf-harness['"]/m,
    );
  });

  it('preload.ts exposes the MAENIFOLD_UAT runtime flag via contextBridge', () => {
    expect(preloadTs).toMatch(/contextBridge\.exposeInMainWorld\(['"]__MAENIFOLD_RUNTIME__['"]/);
    expect(preloadTs).toMatch(/process\.env\.MAENIFOLD_UAT/);
  });

  it('preload.ts does NOT import the harness module', () => {
    // The harness must never leak into the preload bundle.
    expect(preloadTs).not.toMatch(/cdp-perf-harness/);
  });
});
