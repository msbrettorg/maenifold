import { join } from 'node:path';
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

const result = checkNodeModulesIntegrity({
  packageLockPath,
  nodeModulesDirectory,
  fileSystem,
});

if (!result.missing.includes('acorn')) {
  throw new Error('expected acorn to be reported missing');
}

const formatted = formatIntegrityResult(result);
if (formatted.exitCode !== 1) {
  throw new Error(`expected exit code 1, got ${formatted.exitCode}`);
}

process.stderr.write(formatted.output);
process.exitCode = formatted.exitCode;
