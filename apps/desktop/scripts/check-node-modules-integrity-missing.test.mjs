import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { checkNodeModulesIntegrity, formatIntegrityResult } from './check-node-modules-integrity.mjs';

const packageLockPath = '/fixture/app/package-lock.json';
const nodeModulesDirectory = '/fixture/app/node_modules';
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

const fileSystem = {
  readFileSync(path) {
    if (path !== packageLockPath) {
      throw new Error(`unexpected read path: ${path}`);
    }
    return JSON.stringify(fixtureLock);
  },
  existsSync(path) {
    return path === presentPackageDirectory;
  },
  statSync(path) {
    if (path !== presentPackageDirectory) {
      throw new Error(`unexpected stat path: ${path}`);
    }
    return {
      isDirectory() {
        return true;
      },
    };
  },
};

describe('check node_modules integrity missing-package detection', () => {
  it('reports missing package and exits 1', () => {
    const result = checkNodeModulesIntegrity({
      packageLockPath,
      nodeModulesDirectory,
      fileSystem,
    });

    expect(result.missing).toContain('acorn');

    const formatted = formatIntegrityResult(result);
    expect(formatted.exitCode).toBe(1);

    process.stderr.write(formatted.output);
  });
});
