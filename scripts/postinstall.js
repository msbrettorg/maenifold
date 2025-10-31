#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const os = require('os');

console.log('Setting up Maenifold...');

// Ensure binary has correct permissions on Unix-like systems
if (os.platform() !== 'win32') {
  const platform = os.platform();
  const arch = os.arch();

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
  }

  // Try to find binary in optional platform package
  let binaryPath = null;
  try {
    const platformPackagePath = require.resolve(`${platformPackageName}/package.json`);
    const platformRoot = path.dirname(platformPackagePath);
    binaryPath = path.join(platformRoot, 'Maenifold');
  } catch (err) {
    // Platform package not found, try local bin directory (development)
    binaryPath = path.join(__dirname, '..', 'bin', runtimeId, 'Maenifold');
  }

  if (binaryPath && fs.existsSync(binaryPath)) {
    try {
      fs.chmodSync(binaryPath, '755');
      console.log('✓ Binary permissions set');
    } catch (err) {
      console.warn('Warning: Could not set binary permissions:', err.message);
    }
  }
}

// Create default Maenifold directory if it doesn't exist
const defaultRoot = path.join(os.homedir(), 'maenifold');
if (!fs.existsSync(defaultRoot)) {
  try {
    fs.mkdirSync(defaultRoot, { recursive: true });
    fs.mkdirSync(path.join(defaultRoot, 'memory'), { recursive: true });
    console.log(`✓ Created default directory: ${defaultRoot}`);
  } catch (err) {
    console.warn('Warning: Could not create default directory:', err.message);
  }
}

console.log('\nMaenifold installation complete!');
console.log('\nUsage:');
console.log('  CLI mode:  npx maenifold --tool memorystatus --payload \'{}\'');
console.log('  MCP mode:  npx maenifold --mcp');
console.log('\nFor MCP configuration, add to your Claude Desktop config:');
console.log(JSON.stringify({
  "mcpServers": {
    "maenifold": {
      "command": "npx",
      "args": ["maenifold", "--mcp"],
      "env": {
        "MAENIFOLD_ROOT": "~/maenifold"
      }
    }
  }
}, null, 2));
