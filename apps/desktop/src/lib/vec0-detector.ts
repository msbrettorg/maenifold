export interface Vec0ReleaseMapping {
  platform: 'darwin' | 'linux' | 'win32';
  arch: 'arm64' | 'x64';
  assetPattern: string;
  platformDir: string;
  filename: string;
}

const VEC0_RELEASE_MAPPINGS: Vec0ReleaseMapping[] = [
  {
    platform: 'darwin',
    arch: 'arm64',
    assetPattern: 'sqlite-vec-*-loadable-macos-aarch64.tar.gz',
    platformDir: 'osx-arm64',
    filename: 'vec0.dylib',
  },
  {
    platform: 'darwin',
    arch: 'x64',
    assetPattern: 'sqlite-vec-*-loadable-macos-x86_64.tar.gz',
    platformDir: 'osx-x64',
    filename: 'vec0.dylib',
  },
  {
    platform: 'linux',
    arch: 'arm64',
    assetPattern: 'sqlite-vec-*-loadable-linux-aarch64.tar.gz',
    platformDir: 'linux-arm64',
    filename: 'vec0.so',
  },
  {
    platform: 'linux',
    arch: 'x64',
    assetPattern: 'sqlite-vec-*-loadable-linux-x86_64.tar.gz',
    platformDir: 'linux-x64',
    filename: 'vec0.so',
  },
  {
    platform: 'win32',
    arch: 'x64',
    assetPattern: 'sqlite-vec-*-loadable-windows-x86_64.tar.gz',
    platformDir: 'win-x64',
    filename: 'vec0.dll',
  },
];

export function resolveVec0ReleaseMapping(
  platform: string,
  arch: string,
): Vec0ReleaseMapping | null {
  return (
    VEC0_RELEASE_MAPPINGS.find((mapping) => mapping.platform === platform && mapping.arch === arch)
    ?? null
  );
}

export function isVec0Installed(fileSizeBytes: number | null | undefined): boolean {
  return typeof fileSizeBytes === 'number' && fileSizeBytes > 0;
}

export function buildVec0TargetPath(
  mapping: Vec0ReleaseMapping,
  nativeRoot = '~/maenifold/assets/native',
): string {
  return `${nativeRoot}/${mapping.platformDir}/${mapping.filename}`;
}
