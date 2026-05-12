import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';

const electronMock = vi.hoisted(() => ({
  app: {
    on: vi.fn(),
    whenReady: vi.fn(() => ({ then: vi.fn() })),
    getPath: vi.fn(() => '/mock'),
    getAppPath: vi.fn(() => '/mock/app'),
    getVersion: vi.fn(() => '0.0.0'),
    quit: vi.fn(),
  },
  BrowserWindow: Object.assign(vi.fn(), {
    getFocusedWindow: vi.fn(() => null),
    getAllWindows: vi.fn(() => []),
  }),
  dialog: {
    showOpenDialog: vi.fn(),
  },
  ipcMain: { handle: vi.fn() },
  protocol: { registerSchemesAsPrivileged: vi.fn(), handle: vi.fn() },
  shell: { openExternal: vi.fn() },
}));

vi.mock('electron', () => electronMock);
vi.mock('../terminalManager', () => ({
  TerminalManager: class {
    dispose = vi.fn();
  },
  validateCreateTerminalParams: vi.fn(),
  validateTerminalOperationParams: vi.fn(),
}));
vi.mock('../acp-file-system', () => ({ registerAcpFileSystemHandlers: vi.fn() }));
vi.mock('../config-file', () => ({ registerConfigHandlers: vi.fn() }));
vi.mock('../vec0', () => ({ registerVec0Handlers: vi.fn() }));
vi.mock('../voice', () => ({ registerVoiceHandlers: vi.fn() }));
vi.mock('../permission-ledger', () => ({ registerPermissionLedgerHandlers: vi.fn() }));

let openDirectoryDialog: () => Promise<string | null>;

beforeAll(async () => {
  const mod = await import('../main.js');
  openDirectoryDialog = mod.openDirectoryDialog;
});

afterEach(() => {
  delete process.env.MAENIFOLD_UAT;
  delete process.env.MAENIFOLD_UAT_FOLDER_PICKER_PATH;
  vi.mocked(electronMock.dialog.showOpenDialog).mockReset();
});

describe('openDirectoryDialog', () => {
  it('returns the fixture path when MAENIFOLD_UAT=1 and path env is set', async () => {
    process.env.MAENIFOLD_UAT = '1';
    process.env.MAENIFOLD_UAT_FOLDER_PICKER_PATH = '/tmp/uat-fixture';

    const result = await openDirectoryDialog();

    expect(result).toBe('/tmp/uat-fixture');
    expect(electronMock.dialog.showOpenDialog).not.toHaveBeenCalled();
  });

  it('returns null when MAENIFOLD_UAT=1 but no fixture path is set', async () => {
    process.env.MAENIFOLD_UAT = '1';

    const result = await openDirectoryDialog();

    expect(result).toBeNull();
    expect(electronMock.dialog.showOpenDialog).not.toHaveBeenCalled();
  });

  it('calls dialog.showOpenDialog and returns path when MAENIFOLD_UAT is unset', async () => {
    vi.mocked(electronMock.dialog.showOpenDialog).mockResolvedValue({
      canceled: false,
      filePaths: ['/some/path'],
    } as Awaited<ReturnType<typeof electronMock.dialog.showOpenDialog>>);

    const result = await openDirectoryDialog();

    expect(electronMock.dialog.showOpenDialog).toHaveBeenCalled();
    expect(result).toBe('/some/path');
  });
});
