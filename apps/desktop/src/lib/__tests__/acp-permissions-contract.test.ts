import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import * as Module from '@/lib/acp-permissions';

const libDir = resolve(dirname(fileURLToPath(import.meta.url)), '..');

function lineCount(name: string): number {
  const content = readFileSync(resolve(libDir, name), 'utf8');
  return content.split(/\r?\n/).length - Number(content.endsWith('\n'));
}

function sourceExports(name: string): string[] {
  return [...readFileSync(resolve(libDir, name), 'utf8').matchAll(/^export\s+(?:type\s+)?(?:class|const|function|interface|type)\s+(\w+)/gm)]
    .map(([, exportName]) => exportName)
    .sort();
}

describe('acp-permissions split contract', () => {
  it('acp-permissions.ts is ≤300 LOC', () => {
    expect(lineCount('acp-permissions.ts')).toBeLessThanOrEqual(300);
  });

  it('lib/ has acp-permission-policy.ts helper', () => {
    expect(existsSync(resolve(libDir, 'acp-permission-policy.ts'))).toBe(true);
  });

  it('lib/ has acp-permission-prompts.ts helper', () => {
    expect(existsSync(resolve(libDir, 'acp-permission-prompts.ts'))).toBe(true);
  });

  it('each new helper is ≤300 LOC', () => {
    for (const name of ['acp-permission-policy.ts', 'acp-permission-prompts.ts']) {
      expect(existsSync(resolve(libDir, name)), `${name} should exist`).toBe(true);
      expect(lineCount(name), `${name} should stay within the 300 LOC cap`).toBeLessThanOrEqual(300);
    }
  });

  it('acp-permissions module exports the locked public surface', () => {
    expect(sourceExports('acp-permissions.ts')).toMatchInlineSnapshot(`
      [
        "InlinePermissionMessageSink",
        "PermissionAgentNonCompliantError",
        "PermissionHandler",
      ]
    `);
    expect(Object.keys(Module).sort()).toMatchInlineSnapshot(`
      [
        "PermissionAgentNonCompliantError",
        "PermissionHandler",
      ]
    `);
  });
});
