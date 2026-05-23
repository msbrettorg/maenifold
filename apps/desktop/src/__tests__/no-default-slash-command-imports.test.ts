import { readdirSync, readFileSync } from 'node:fs';
import { extname, join, relative } from 'node:path';
import { describe, expect, it } from 'vitest';

function collectSourceFiles(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const entryPath = join(dir, entry.name);
    if (entry.isDirectory()) {
      return collectSourceFiles(entryPath);
    }

    return ['.ts', '.vue'].includes(extname(entry.name)) ? [entryPath] : [];
  });
}

describe('desktop source imports', () => {
  it('does not import deleted default slash command modules anywhere under src', () => {
    const srcRoot = join(process.cwd(), 'src');
    const forbiddenImportPatterns = [
      /import\s+[^;]*from\s+['"][^'"]*default-commands['"]/,
      /import\s+[^;]*from\s+['"][^'"]*useBuiltInSlashCommands['"]/,
    ];

    const matches = collectSourceFiles(srcRoot).flatMap((filePath) => {
      const source = readFileSync(filePath, 'utf8');

      return forbiddenImportPatterns.flatMap((pattern) => {
        const match = source.match(pattern);
        return match ? [`${relative(srcRoot, filePath)}: ${match[0]}`] : [];
      });
    });

    expect(matches).toEqual([]);
  });
});
