import { spawn, spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { homedir } from 'node:os';
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

const WINDOWS_SYSTEM32 = 'C:\\Windows\\System32';
const WINDOWS_TASKLIST = `${WINDOWS_SYSTEM32}\\tasklist.exe`;
const MACOS_OPEN = '/usr/bin/open';
const POSIX_PGREP = '/usr/bin/pgrep';

function findHandyPath(): string | null {
  const windowsHome = process.platform === 'win32' ? homedir() : '';
  const candidates = process.platform === 'win32'
    ? [
        windowsHome ? `${windowsHome}\\AppData\\Local\\Programs\\Handy\\Handy.exe` : '',
        windowsHome ? `${windowsHome}\\AppData\\Local\\Handy\\handy.exe` : '',
        'C:\\Program Files\\Handy\\Handy.exe',
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
        WINDOWS_TASKLIST,
        ['/FI', 'IMAGENAME eq handy.exe', '/NH'],
        { encoding: 'utf8', timeout: 3_000 },
      );
      return /handy\.exe/i.test(result.stdout || '');
    }

    const handyResult = spawnSync(
      POSIX_PGREP,
      ['-x', 'Handy'],
      { encoding: 'utf8', timeout: 3_000 },
    );
    if (handyResult.status === 0) {
      return true;
    }

    const lowerCaseResult = spawnSync(
      POSIX_PGREP,
      ['-x', 'handy'],
      { encoding: 'utf8', timeout: 3_000 },
    );
    return lowerCaseResult.status === 0;
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
            spawnSync(MACOS_OPEN, ['-a', 'Handy'], { timeout: 3_000 });
          } catch {
            /* best effort */
          }
        }

        return { ok: true, alreadyRunning: true };
      }

      const child = spawn(exePath, [], { detached: true, stdio: 'ignore' });
      child.unref();
      return { ok: true, pid: child.pid };
    } catch {
      return { ok: false, error: 'Failed to launch Handy.' };
    }
  });
}
