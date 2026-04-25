import { describe, expect, it } from 'vitest';

import {
  buildVec0TargetPath,
  isVec0Installed,
  resolveVec0ReleaseMapping,
} from '../vec0-detector';

describe('vec0-detector', () => {
  it('maps darwin arm64 to the macOS aarch64 release asset', () => {
    expect(resolveVec0ReleaseMapping('darwin', 'arm64')).toEqual({
      platform: 'darwin',
      arch: 'arm64',
      assetPattern: 'sqlite-vec-*-loadable-macos-aarch64.tar.gz',
      platformDir: 'osx-arm64',
      filename: 'vec0.dylib',
    });
  });

  it('returns null for unsupported platforms', () => {
    expect(resolveVec0ReleaseMapping('win32', 'arm64')).toBeNull();
  });

  it('treats zero-byte binaries as missing', () => {
    expect(isVec0Installed(0)).toBe(false);
    expect(isVec0Installed(32)).toBe(true);
  });

  it('builds the shared native target path', () => {
    const mapping = resolveVec0ReleaseMapping('linux', 'x64');
    expect(mapping).not.toBeNull();
    expect(buildVec0TargetPath(mapping!)).toBe('~/maenifold/assets/native/linux-x64/vec0.so');
  });
});
