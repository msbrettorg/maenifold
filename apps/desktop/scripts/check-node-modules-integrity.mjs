import * as fs from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const defaultAppDirectory = resolve(scriptDirectory, '..');

function hasPlatformFilter(metadata) {
  return Object.hasOwn(metadata, 'os')
    || Object.hasOwn(metadata, 'cpu')
    || metadata.optional === true;
}

function isRootNodeModulesEntry(packagePath) {
  return (
    packagePath.startsWith('node_modules/')
    && !packagePath.slice('node_modules/'.length).includes('/node_modules/')
  );
}

function packageNameFromLockPath(packagePath) {
  return packagePath.slice('node_modules/'.length);
}

function directoryExists(fileSystem, directoryPath) {
  if (!fileSystem.existsSync(directoryPath)) {
    return false;
  }

  try {
    return fileSystem.statSync(directoryPath).isDirectory();
  } catch {
    return false;
  }
}

export function checkNodeModulesIntegrity({
  packageLockPath,
  nodeModulesDirectory,
  fileSystem = fs,
}) {
  const lockFile = JSON.parse(fileSystem.readFileSync(packageLockPath, 'utf8'));
  const packages = lockFile.packages ?? {};
  const missing = [];
  let checked = 0;
  let skippedPlatformSpecific = 0;

  for (const [packagePath, metadata] of Object.entries(packages)) {
    if (!isRootNodeModulesEntry(packagePath)) {
      continue;
    }

    if (hasPlatformFilter(metadata)) {
      skippedPlatformSpecific += 1;
      continue;
    }

    checked += 1;
    const packageName = packageNameFromLockPath(packagePath);
    const packageDirectory = join(nodeModulesDirectory, packageName);
    if (!directoryExists(fileSystem, packageDirectory)) {
      missing.push(packageName);
    }
  }

  const nestedNodeModulesDirectory = join(nodeModulesDirectory, 'node_modules');
  const hasNestedContamination = fileSystem.existsSync(nestedNodeModulesDirectory);

  return {
    checked,
    skippedPlatformSpecific,
    missing,
    hasNestedContamination,
  };
}

export function formatIntegrityResult(result) {
  const failures = [];

  for (const packageName of result.missing) {
    failures.push(`missing: ${packageName}`);
  }

  if (result.hasNestedContamination) {
    failures.push('contamination: node_modules/node_modules');
  }

  if (failures.length > 0) {
    return {
      exitCode: 1,
      output: `${failures.join('\n')}\n`,
    };
  }

  return {
    exitCode: 0,
    output: `node_modules integrity ok: checked ${result.checked} packages, skipped ${result.skippedPlatformSpecific} platform-specific packages\n`,
  };
}

export function runNodeModulesIntegrityCheck({
  appDirectory = defaultAppDirectory,
  fileSystem = fs,
} = {}) {
  const packageLockPath = join(appDirectory, 'package-lock.json');
  const nodeModulesDirectory = join(appDirectory, 'node_modules');
  return formatIntegrityResult(checkNodeModulesIntegrity({
    packageLockPath,
    nodeModulesDirectory,
    fileSystem,
  }));
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const result = runNodeModulesIntegrityCheck();
  const writer = result.exitCode === 0 ? process.stdout : process.stderr;
  writer.write(result.output);
  process.exitCode = result.exitCode;
}
