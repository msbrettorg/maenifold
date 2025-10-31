#!/usr/bin/env node

/**
 * Maenifold NPM wrapper script
 * Detects platform and runs the appropriate binary
 */

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');
const os = require('os');

function getBinaryPath() {
  const platform = os.platform();
  const arch = os.arch();

  let binaryName = 'Maenifold';
  let platformPackageName = '';
  let runtimeId = '';

  if (platform === 'darwin') {
    if (arch === 'arm64') {
      platformPackageName = 'maenifold-darwin-arm64';
      runtimeId = 'osx-arm64';
    } else {
      platformPackageName = 'maenifold-darwin-x64';
      runtimeId = 'osx-x64';
    }
  } else if (platform === 'linux') {
    if (arch === 'arm64') {
      platformPackageName = 'maenifold-linux-arm64';
      runtimeId = 'linux-arm64';
    } else {
      platformPackageName = 'maenifold-linux-x64';
      runtimeId = 'linux-x64';
    }
  } else if (platform === 'win32') {
    binaryName = 'Maenifold.exe';
    if (arch === 'arm64') {
      platformPackageName = 'maenifold-win32-arm64';
      runtimeId = 'win-arm64';
    } else {
      platformPackageName = 'maenifold-win32-x64';
      runtimeId = 'win-x64';
    }
  } else {
    console.error(`Unsupported platform: ${platform}-${arch}`);
    process.exit(1);
  }

  // Try to find binary in optional platform package (node_modules)
  try {
    const platformPackagePath = require.resolve(`${platformPackageName}/package.json`);
    const platformRoot = path.dirname(platformPackagePath);
    const binaryPath = path.join(platformRoot, binaryName);

    if (fs.existsSync(binaryPath)) {
      return binaryPath;
    }
  } catch (err) {
    // Platform package not found in node_modules, continue to fallback
  }

  // Fallback to local bin directory (for development or legacy installs)
  const localBinaryPath = path.join(__dirname, '..', 'bin', runtimeId, binaryName);
  if (fs.existsSync(localBinaryPath)) {
    return localBinaryPath;
  }

  // Final fallback to generic binary
  const fallbackPath = path.join(__dirname, '..', 'bin', binaryName);
  if (fs.existsSync(fallbackPath)) {
    return fallbackPath;
  }

  console.error(`Binary not found for ${platform}-${arch}`);
  console.error(`Expected platform package: ${platformPackageName}`);
  console.error('Please ensure the package is properly installed.');
  console.error('Try running: npm install');
  process.exit(1);
}

function main() {
  const binaryPath = getBinaryPath();
  const args = process.argv.slice(2);

  const child = spawn(binaryPath, args, {
    stdio: 'inherit',
    env: process.env
  });

  child.on('error', (err) => {
    console.error('Failed to start maenifold:', err);
    process.exit(1);
  });

  child.on('exit', (code) => {
    process.exit(code ?? 0);
  });
}

main();
