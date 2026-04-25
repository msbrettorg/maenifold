import { mkdir, rename, rm, stat, writeFile } from 'node:fs/promises';
import { basename, join } from 'node:path';
import { gunzipSync } from 'node:zlib';
import { createHash } from 'node:crypto';
import { DatabaseSync } from 'node:sqlite';
import { app, ipcMain } from 'electron';

type SupportedPlatform = 'darwin' | 'linux' | 'win32';
type SupportedArch = 'arm64' | 'x64';

interface Vec0Mapping {
  platform: SupportedPlatform;
  arch: SupportedArch;
  assetPattern: string;
  platformDir: string;
  filename: string;
}

interface GitHubReleaseAsset {
  name: string;
  browser_download_url: string;
}

interface GitHubReleaseResponse {
  assets?: GitHubReleaseAsset[];
}

interface Vec0DownloadProgress {
  current: number;
  total: number;
  filename?: string;
}

export interface Vec0Status {
  platform: string;
  arch: string;
  supported: boolean;
  installed: boolean;
  fileSizeBytes: number;
  assetPattern: string | null;
  targetPath: string | null;
  downloadProgress: Vec0DownloadProgress | null;
}

const VEC0_VERSION = 'v0.1.9'; // Pin to tested version
const RELEASE_URL = `https://api.github.com/repos/asg017/sqlite-vec/releases/tags/${VEC0_VERSION}`;
const ALLOWED_DOWNLOAD_HOSTS = [
  'github.com',
  'objects.githubusercontent.com',
  'github-releases.githubusercontent.com',
  'release-assets.githubusercontent.com',
] as const;
const MAPPINGS: Vec0Mapping[] = [
  { platform: 'darwin', arch: 'arm64', assetPattern: 'sqlite-vec-*-loadable-macos-aarch64.tar.gz', platformDir: 'osx-arm64', filename: 'vec0.dylib' },
  { platform: 'darwin', arch: 'x64', assetPattern: 'sqlite-vec-*-loadable-macos-x86_64.tar.gz', platformDir: 'osx-x64', filename: 'vec0.dylib' },
  { platform: 'linux', arch: 'arm64', assetPattern: 'sqlite-vec-*-loadable-linux-aarch64.tar.gz', platformDir: 'linux-arm64', filename: 'vec0.so' },
  { platform: 'linux', arch: 'x64', assetPattern: 'sqlite-vec-*-loadable-linux-x86_64.tar.gz', platformDir: 'linux-x64', filename: 'vec0.so' },
  { platform: 'win32', arch: 'x64', assetPattern: 'sqlite-vec-*-loadable-windows-x86_64.tar.gz', platformDir: 'win-x64', filename: 'vec0.dll' },
];
const KNOWN_HASHES: Record<string, string | null> = {
  'darwin-arm64': null, // Set after first verified download
  'darwin-x64': null,
  'linux-arm64': null,
  'linux-x64': null,
  'win32-x64': null,
};

let activeProgress: Vec0DownloadProgress | null = null;

function resolveMapping(): Vec0Mapping | null {
  return MAPPINGS.find(
    (mapping) => mapping.platform === process.platform && mapping.arch === process.arch,
  ) ?? null;
}

function maenifoldRoot(): string {
  return process.env.MAENIFOLD_ROOT || join(app.getPath('home'), 'maenifold');
}

function targetPathFor(mapping: Vec0Mapping): string {
  return join(maenifoldRoot(), 'assets', 'native', mapping.platformDir, mapping.filename);
}

function tempPathFor(targetPath: string): string {
  return `${targetPath}.tmp`;
}

async function statFileSize(path: string): Promise<number> {
  try {
    const file = await stat(path);
    return file.size;
  } catch {
    return 0;
  }
}

async function getStatus(): Promise<Vec0Status> {
  const mapping = resolveMapping();
  const targetPath = mapping ? targetPathFor(mapping) : null;
  const fileSizeBytes = targetPath ? await statFileSize(targetPath) : 0;
  return {
    platform: process.platform,
    arch: process.arch,
    supported: mapping !== null,
    installed: fileSizeBytes > 0,
    fileSizeBytes,
    assetPattern: mapping?.assetPattern ?? null,
    targetPath,
    downloadProgress: activeProgress,
  };
}

function wildcardToRegExp(pattern: string): RegExp {
  const escaped = pattern.replace(/[.+?^${}()|[\]\\]/g, '\\$&').replace(/\*/g, '.*');
  return new RegExp(`^${escaped}$`);
}

function validateDownloadUrl(url: string): void {
  const parsed = new URL(url);
  if (parsed.protocol !== 'https:') {
    throw new Error(`Unsafe download protocol: ${parsed.protocol}`);
  }
  if (!ALLOWED_DOWNLOAD_HOSTS.includes(parsed.hostname as (typeof ALLOWED_DOWNLOAD_HOSTS)[number])) {
    throw new Error(`Untrusted download host: ${parsed.hostname}`);
  }
}

async function fetchLatestAsset(mapping: Vec0Mapping): Promise<GitHubReleaseAsset> {
  const response = await fetch(RELEASE_URL, {
    headers: {
      Accept: 'application/vnd.github+json',
      'User-Agent': 'maenifold-desktop',
    },
  });
  if (!response.ok) {
    throw new Error(`GitHub release lookup failed (${response.status})`);
  }
  const release = await response.json() as GitHubReleaseResponse;
  const asset = release.assets?.find((candidate) =>
    wildcardToRegExp(mapping.assetPattern).test(candidate.name),
  );
  if (!asset) {
    throw new Error(`No vec0 release asset matched ${mapping.assetPattern}`);
  }
  return asset;
}

async function downloadArchive(url: string, filename: string): Promise<Buffer> {
  validateDownloadUrl(url);
  const response = await fetch(url, {
    headers: { 'User-Agent': 'maenifold-desktop' },
  });
  validateDownloadUrl(response.url);
  if (!response.ok || !response.body) {
    throw new Error(`Vec0 download failed (${response.status})`);
  }

  const total = Number.parseInt(response.headers.get('content-length') ?? '0', 10) || 0;
  const reader = response.body.getReader();
  const chunks: Uint8Array[] = [];
  let current = 0;
  activeProgress = { current, total, filename };

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    if (!value) continue;
    chunks.push(value);
    current += value.byteLength;
    activeProgress = { current, total, filename };
  }

  return Buffer.concat(chunks.map((chunk) => Buffer.from(chunk)));
}

function verifyArchiveIntegrity(archive: Buffer, mapping: Vec0Mapping): void {
  const hash = createHash('sha256').update(archive).digest('hex');
  const key = `${mapping.platform}-${mapping.arch}`;
  const expected = KNOWN_HASHES[key];
  if (expected === undefined) {
    console.warn(`[vec0] No pinned SHA-256 for ${key}. Computed SHA-256: ${hash}.`);
    return;
  }
  if (expected === null) {
    console.warn(`[vec0] First download for ${key}. SHA-256: ${hash}. Pin this hash in KNOWN_HASHES.`);
    return;
  }
  if (hash !== expected) {
    throw new Error(
      `Vec0 archive integrity check failed: expected ${expected}, got ${hash}`,
    );
  }
}

function parseOctal(raw: Buffer): number {
  const value = raw.toString('utf8').replace(/\0/g, '').trim();
  return value ? Number.parseInt(value, 8) : 0;
}

function extractTarEntry(archive: Buffer, filename: string): Buffer {
  let offset = 0;
  while (offset + 512 <= archive.length) {
    const header = archive.subarray(offset, offset + 512);
    if (header.every((byte) => byte === 0)) {
      break;
    }

    const name = header.subarray(0, 100).toString('utf8').replace(/\0.*$/, '');
    const prefix = header.subarray(345, 500).toString('utf8').replace(/\0.*$/, '');
    const entryName = [prefix, name].filter(Boolean).join('/');
    const size = parseOctal(header.subarray(124, 136));
    const bodyStart = offset + 512;
    const bodyEnd = bodyStart + size;
    const fileBody = archive.subarray(bodyStart, bodyEnd);

    if (basename(entryName) === filename) {
      return Buffer.from(fileBody);
    }

    offset = bodyStart + Math.ceil(size / 512) * 512;
  }

  throw new Error(`Vec0 archive did not contain ${filename}`);
}

function verifyLoadable(targetPath: string): void {
  const db = new DatabaseSync(':memory:');
  try {
    db.enableLoadExtension(true);
    db.loadExtension(targetPath);
    db.enableLoadExtension(false);
    db.exec(`
      CREATE VIRTUAL TABLE temp.vec0_verify USING vec0(
        rowid TEXT PRIMARY KEY,
        embedding FLOAT[1]
      );
      DROP TABLE temp.vec0_verify;
    `);
  } finally {
    db.close();
  }
}

async function downloadVec0(): Promise<Vec0Status> {
  const mapping = resolveMapping();
  if (!mapping) {
    throw new Error(`Unsupported vec0 platform: ${process.platform}/${process.arch}`);
  }

  const targetPath = targetPathFor(mapping);
  const targetDir = join(maenifoldRoot(), 'assets', 'native', mapping.platformDir);
  const tempPath = tempPathFor(targetPath);
  const existingSize = await statFileSize(targetPath);
  if (existingSize > 0) {
    return getStatus();
  }

  const asset = await fetchLatestAsset(mapping);
  const archive = await downloadArchive(asset.browser_download_url, asset.name);
  verifyArchiveIntegrity(archive, mapping);

  try {
    const extracted = extractTarEntry(gunzipSync(archive), mapping.filename);
    await mkdir(targetDir, { recursive: true });
    await rm(tempPath, { force: true });
    await writeFile(tempPath, extracted);
    verifyLoadable(tempPath);
    await rename(tempPath, targetPath);
    return await getStatus();
  } catch (error) {
    await rm(tempPath, { force: true });
    throw error;
  } finally {
    activeProgress = null;
  }
}

export function registerVec0Handlers(): void {
  ipcMain.handle('vec0:status', () => getStatus());
  ipcMain.handle('vec0:download', () => downloadVec0());
}
