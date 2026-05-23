import { readdirSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import * as Module from '@/lib/tool-palette-catalog';

const EXPECTED_RUNTIME_EXPORTS = [
  'TOOL_PALETTE_CATEGORIES',
  'buildToolPaletteEntries',
  'categorizeTools',
  'getPaletteEntrySearchFields',
  'scoredSearch',
].sort();
const EXPECTED_SOURCE_EXPORTS = [
  'TOOL_PALETTE_CATEGORIES',
  'ToolPaletteCategory',
  'ToolPaletteEntry',
  'buildToolPaletteEntries',
  'categorizeTools',
  'getPaletteEntrySearchFields',
  'scoredSearch',
].sort();
const TEST_DIR = dirname(fileURLToPath(import.meta.url));
const LIB_DIR = resolve(TEST_DIR, '..');
const CATALOG_PATH = resolve(LIB_DIR, 'tool-palette-catalog.ts');
const countLines = (source: string) => source.split('\n').length - (source.endsWith('\n') ? 1 : 0);
const sourceExports = (source: string) => [...source.matchAll(/^export\s+(?:const|function|interface|type)\s+([A-Za-z0-9_]+)/gm)]
  .map(([, name]) => name)
  .sort();

describe('tool-palette-catalog split contract', () => {
  it('tool-palette-catalog.ts is ≤300 LOC', () => {
    expect(countLines(readFileSync(CATALOG_PATH, 'utf8'))).toBeLessThanOrEqual(300);
  });

  it('module exports the locked public surface', () => {
    const source = readFileSync(CATALOG_PATH, 'utf8');
    expect(Object.keys(Module).sort()).toEqual(EXPECTED_RUNTIME_EXPORTS);
    expect([...new Set(sourceExports(source))]).toEqual(EXPECTED_SOURCE_EXPORTS);
  });

  it('every .ts file matching tool-palette-*.ts in lib/ is ≤300 LOC', () => {
    const oversized = readdirSync(LIB_DIR)
      .filter((name) => name.startsWith('tool-palette-') && name.endsWith('.ts') && !name.includes('.test.'))
      .map((name) => ({ name, lines: countLines(readFileSync(resolve(LIB_DIR, name), 'utf8')) }))
      .filter(({ lines }) => lines > 300);
    expect(oversized).toEqual([]);
  });
});
