import { spawn, spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { ipcMain } from 'electron';

type VoiceCheckResult = {
  installed: boolean;
  path: string | null;
  running: boolean;
  installHint: string;
};

type VoiceLaunchResult = {
  ok: boolean;
  error?: string;
  alreadyRunning?: boolean;
  pid?: number;
};

function findHandyPath(): string | null {
  const candidates = process.platform === 'win32'
    ? [
        `${process.env.LOCALAPPDATA || ''}\\Programs\\Handy\\Handy.exe`,
        `${process.env.LOCALAPPDATA || ''}\\Handy\\handy.exe`,
        `${process.env.PROGRAMFILES || ''}\\Handy\\Handy.exe`,
      ]
    : process.platform === 'darwin'
      ? ['/Applications/Handy.app/Contents/MacOS/Handy']
      : ['/usr/bin/handy', '/usr/local/bin/handy'];

  for (const candidate of candidates) {
    if (candidate && existsSync(candidate)) {
      return candidate;
    }
  }

  return null;
}

function isHandyRunning(): boolean {
  try {
    if (process.platform === 'win32') {
      const result = spawnSync(
        'tasklist',
        ['/FI', 'IMAGENAME eq handy.exe', '/NH'],
        { encoding: 'utf8', timeout: 3_000 },
      );
      return /handy\.exe/i.test(result.stdout || '');
    }

    const result = spawnSync(
      'sh',
      ['-c', 'pgrep -x Handy || pgrep -x handy'],
      { encoding: 'utf8', timeout: 3_000 },
    );
    return result.status === 0;
  } catch {
    return false;
  }
}

const installHint = process.platform === 'darwin'
  ? 'brew install --cask handy'
  : process.platform === 'win32'
    ? 'winget install cjpais.Handy'
    : 'https://github.com/cjpais/handy/releases';

export function registerVoiceHandlers(): void {
  ipcMain.handle('voice:check', async (): Promise<VoiceCheckResult> => {
    const handyPath = findHandyPath();
    return {
      installed: !!handyPath,
      path: handyPath,
      running: isHandyRunning(),
      installHint,
    };
  });

  ipcMain.handle('voice:launch', async (): Promise<VoiceLaunchResult> => {
    const exePath = findHandyPath();
    if (!exePath) {
      return { ok: false, error: `Handy not installed. Install via: ${installHint}` };
    }

    try {
      if (isHandyRunning()) {
        if (process.platform === 'darwin') {
          try {
            spawnSync('open', ['-a', 'Handy'], { timeout: 3_000 });
          } catch {
            /* best effort */
          }
        }

        return { ok: true, alreadyRunning: true };
      }

      const child = spawn(exePath, [], { detached: true, stdio: 'ignore' });
      child.unref();
      return { ok: true, pid: child.pid };
    } catch (error: unknown) {
      return { ok: false, error: error instanceof Error ? error.message : String(error) };
    }
  });
}
