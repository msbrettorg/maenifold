import { contextBridge, ipcRenderer, type IpcRendererEvent } from 'electron';

type Vec0Status = {
  platform: string;
  arch: string;
  supported: boolean;
  installed: boolean;
  fileSizeBytes: number;
  assetPattern: string | null;
  targetPath: string | null;
  downloadProgress: {
    current: number;
    total: number;
    filename?: string;
  } | null;
};

type PermissionLedgerPayload = {
  request: unknown;
  context: unknown;
};

type PermissionResolveDecisionPayload = {
  token: string;
  optionId: string;
  scope?: 'session' | 'agent' | 'forever';
};

type WritePermissionPromptPayload = {
  requestId: string;
  path: string;
  sessionId?: string;
  contentPreview: string;
};

type WritePermissionResolvePayload = {
  requestId: string;
  decision: 'allow_once' | 'allow_always' | 'reject';
};

type TerminalCreatePayload = {
  sessionId?: string;
  command: string;
  args?: string[];
  env?: Array<{ name: string; value: string }>;
  cwd?: string | null;
  outputByteLimit?: number | null;
};

type TerminalIdPayload = {
  sessionId?: string;
  terminalId: string;
};

type TerminalExitStatus = {
  exitCode?: number | null;
  signal?: string | null;
};

const electronAPI = {
  openDirectory: () => ipcRenderer.invoke('dialog:open-directory') as Promise<string | null>,
  getAppVersion: () => ipcRenderer.invoke('app:get-version') as Promise<string>,
  quit: () => ipcRenderer.invoke('app:quit') as Promise<void>,
  readTextFile: (params: { path: string; line?: number; limit?: number }) =>
    ipcRenderer.invoke('acp:read-text-file', params) as Promise<{ content: string }>,
  writeTextFile: (params: { path: string; content: string; sessionId?: string }) =>
    ipcRenderer.invoke('acp:write-text-file', params) as Promise<Record<string, never>>,
  onWritePermissionPrompt: (handler: (payload: WritePermissionPromptPayload) => void) => {
    const listener = (_event: IpcRendererEvent, payload: WritePermissionPromptPayload) =>
      handler(payload);
    ipcRenderer.on('acp:write-permission:prompt', listener);
    return () => ipcRenderer.removeListener('acp:write-permission:prompt', listener);
  },
  resolveWritePermission: (payload: WritePermissionResolvePayload) =>
    ipcRenderer.invoke('acp:write-permission:resolve', payload) as Promise<{ ok: true }>,
  checkEnvVarPresence: (names: string[]) =>
    ipcRenderer.invoke('check-env-vars', names) as Promise<Record<string, boolean>>,
  gateSetRoot: (params: { root: string | null }) =>
    ipcRenderer.invoke('gate:set-root', params) as Promise<Record<string, never>>,
  configRead: () =>
    ipcRenderer.invoke('config:read') as Promise<{ path: string; content: string; exists: boolean }>,
  configWrite: (content: string) =>
    ipcRenderer.invoke('config:write', content) as Promise<{ path: string }>,
  configPath: () => ipcRenderer.invoke('config:path') as Promise<string>,
  onConfigChanged: (handler: () => void) => {
    const listener = () => handler();
    ipcRenderer.on('config:changed', listener);
    return () => ipcRenderer.removeListener('config:changed', listener);
  },
  vec0Status: () => ipcRenderer.invoke('vec0:status') as Promise<Vec0Status>,
  vec0Download: () => ipcRenderer.invoke('vec0:download') as Promise<Vec0Status>,
  voiceCheck: () => ipcRenderer.invoke('voice:check') as Promise<{
    installed: boolean;
    path: string | null;
    running: boolean;
  }>,
  voiceLaunch: () => ipcRenderer.invoke('voice:launch') as Promise<{
    ok: boolean;
    error?: string;
    alreadyRunning?: boolean;
    pid?: number;
  }>,
  permissionTryAutoDecide: (payload: PermissionLedgerPayload) =>
    ipcRenderer.invoke('permission:try-auto-decide', payload) as Promise<unknown>,
  permissionResolveDecision: (payload: PermissionResolveDecisionPayload) =>
    ipcRenderer.invoke('permission:resolve-decision', payload) as Promise<void>,
  permissionListDecisions: () => ipcRenderer.invoke('permission:list-decisions') as Promise<unknown[]>,
  permissionListAuditEvents: () =>
    ipcRenderer.invoke('permission:list-audit-events') as Promise<unknown[]>,
  permissionRevokeDecision: (id: number) =>
    ipcRenderer.invoke('permission:revoke-decision', id) as Promise<void>,
  permissionResetConfirmPrepare: () =>
    ipcRenderer.invoke('permission:reset-confirm-prepare') as Promise<string>,
  permissionResetDecisions: (token: string) =>
    ipcRenderer.invoke('permission:reset-decisions', token) as Promise<void>,
  terminal: {
    create: (params: TerminalCreatePayload) =>
      ipcRenderer.invoke('terminal:create', params) as Promise<{
        terminalId: string;
        outputByteLimit: number;
      }>,
    getOutput: (params: TerminalIdPayload) =>
      ipcRenderer.invoke('terminal:output', params) as Promise<{
        output: string;
        truncated: boolean;
        exitStatus?: TerminalExitStatus | null;
      }>,
    waitForExit: (params: TerminalIdPayload) =>
      ipcRenderer.invoke('terminal:wait_for_exit', params) as Promise<TerminalExitStatus>,
    kill: (params: TerminalIdPayload) =>
      ipcRenderer.invoke('terminal:kill', params) as Promise<Record<string, never>>,
    release: (params: TerminalIdPayload) =>
      ipcRenderer.invoke('terminal:release', params) as Promise<Record<string, never>>,
    onOutput: (handler: (terminalId: string, chunk: string) => void) => {
      const listener = (_event: IpcRendererEvent, terminalId: string, chunk: string) =>
        handler(terminalId, chunk);
      ipcRenderer.on('terminal:output', listener);
      return () => ipcRenderer.removeListener('terminal:output', listener);
    },
  },
};

contextBridge.exposeInMainWorld('electronAPI', electronAPI);
contextBridge.exposeInMainWorld('api', { quit: electronAPI.quit });
