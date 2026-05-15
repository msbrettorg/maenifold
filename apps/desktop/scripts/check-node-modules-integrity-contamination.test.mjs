import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { checkNodeModulesIntegrity, formatIntegrityResult } from './check-node-modules-integrity.mjs';

const packageLockPath = '/fixture/app/package-lock.json';
const nodeModulesDirectory = '/fixture/app/node_modules';
const nestedNodeModulesDirectory = join(nodeModulesDirectory, 'node_modules');
const acornDirectory = join(nodeModulesDirectory, 'acorn');
const presentPackageDirectory = join(nodeModulesDirectory, '@scope/present');

const fixtureLock = {
  packages: {
    '': {},
    'node_modules/acorn': {},
    'node_modules/@scope/present': {},
    'node_modules/platform-only': {
      os: ['darwin'],
    },
  },
};

function createFileSystem({ contaminated }) {
  const presentDirectories = new Set([
    acornDirectory,
    presentPackageDirectory,
    contaminated ? nestedNodeModulesDirectory : null,
  ]);

  return {
    readFileSync(path) {
      if (path !== packageLockPath) {
        throw new Error(`unexpected read path: ${path}`);
      }
      return JSON.stringify(fixtureLock);
    },
    existsSync(path) {
      return presentDirectories.has(path);
    },
    statSync(path) {
      if (!presentDirectories.has(path)) {
        throw new Error(`unexpected stat path: ${path}`);
      }
      return {
        isDirectory() {
          return true;
        },
      };
    },
  };
}

describe('check node_modules integrity contamination detection', () => {
  it('fails only for nested node_modules contamination', () => {
    const result = checkNodeModulesIntegrity({
      packageLockPath,
      nodeModulesDirectory,
      fileSystem: createFileSystem({ contaminated: true }),
    });

    expect(result.checked).toBe(2);
    expect(result.skippedPlatformSpecific).toBe(1);
    expect(result.missing).toEqual([]);
    expect(result.hasNestedContamination).toBe(true);

    const formatted = formatIntegrityResult(result);
    expect(formatted.exitCode).toBe(1);
    expect(formatted.output).toContain('contamination: node_modules/node_modules');

    const cleanResult = checkNodeModulesIntegrity({
      packageLockPath,
      nodeModulesDirectory,
      fileSystem: createFileSystem({ contaminated: false }),
    });
    const cleanFormatted = formatIntegrityResult(cleanResult);
    expect(cleanFormatted.exitCode).toBe(0);

    process.stderr.write(formatted.output);
  });
});
