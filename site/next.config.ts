import type { NextConfig } from 'next';
import { execSync } from 'child_process';

let version = 'dev';
try {
  version = execSync('git describe --tags --abbrev=0', { encoding: 'utf-8' }).trim();
} catch {
  // Fallback if no tags exist
}

const nextConfig: NextConfig = {
  output: 'export',
  env: {
    NEXT_PUBLIC_VERSION: version,
  },
};

export default nextConfig;
