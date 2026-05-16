import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { checkNodeModulesIntegrity, formatIntegrityResult } from './check-node-modules-integrity.mjs';

const packageLockPath = '/fixture/app/package-lock.json';
const nodeModulesDirectory = '/fixture/app/node_modules';

const fixtureLock = {
  packages: {
    '': {},
    'node_modules/os-only-pkg': {
      os: ['linux'],
    },
    'node_modules/cpu-only-pkg': {
      cpu: ['x64'],
    },
    'node_modules/optional-only-pkg': {
      optional: true,
    },
    'node_modules/optional-missing-pkg': {
      optional: true,
    },
    'node_modules/normal-pkg': {},
  },
};

function createFileSystem({ existingPackages }) {
  const presentDirectories = new Set(
    existingPackages.map((packageName) => join(nodeModulesDirectory, packageName)),
  );

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

function checkWithPackages(existingPackages) {
  return checkNodeModulesIntegrity({
    packageLockPath,
    nodeModulesDirectory,
    fileSystem: createFileSystem({ existingPackages }),
  });
}

describe('check node_modules integrity optional package skipping', () => {
  it('skips entry with os field only', () => {
    const result = checkWithPackages(['normal-pkg']);

    expect(result.missing).not.toContain('os-only-pkg');
    expect(result.skippedPlatformSpecific).toBeGreaterThanOrEqual(1);
  });

  it('skips entry with cpu field only', () => {
    const result = checkWithPackages(['normal-pkg']);

    expect(result.missing).not.toContain('cpu-only-pkg');
    expect(result.skippedPlatformSpecific).toBeGreaterThanOrEqual(1);
  });

  it('skips entry with optional:true only (no os/cpu)', () => {
    const result = checkWithPackages(['normal-pkg', 'optional-only-pkg']);

    expect(result.missing).not.toContain('optional-only-pkg');
    expect(result.skippedPlatformSpecific).toBeGreaterThanOrEqual(1);
    expect(result.skippedPlatformSpecific).toBe(4);
  });

  it('skips optional:true entry even when missing on disk (exit 0)', () => {
    const result = checkWithPackages(['normal-pkg']);

    expect(result.missing).not.toContain('optional-missing-pkg');
    expect(formatIntegrityResult(result).exitCode).toBe(0);
  });
});
