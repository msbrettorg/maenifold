/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_MCP_BASE_URL?: string;
  readonly VITE_DAEMON_URL?: string;
  readonly VITE_APP_VERSION?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

type ElectronVoiceCheckResult = {
  installed: boolean;
  path: string | null;
  running: boolean;
  installHint: string;
};

type ElectronVoiceLaunchResult = {
  ok: boolean;
  error?: string;
  alreadyRunning?: boolean;
  pid?: number;
};

type ElectronApi = {
  openDirectory?: () => Promise<string | null>;
  getAppVersion?: () => Promise<string>;
  quit?: () => Promise<void>;
  readTextFile?: (params: { path: string; line?: number; limit?: number }) => Promise<{ content: string }>;
  writeTextFile?: (params: { path: string; content: string; sessionId?: string }) => Promise<Record<string, never>>;
  onWritePermissionPrompt?: (handler: (payload: {
    requestId: string;
    path: string;
    sessionId?: string;
    contentPreview: string;
  }) => void) => () => void;
  resolveWritePermission?: (payload: {
    requestId: string;
    decision: 'allow_once' | 'allow_always' | 'reject';
  }) => Promise<{ ok: true }>;
  checkEnvVarPresence?: (names: string[]) => Promise<Record<string, boolean>>;
  gateSetRoot?: (params: { root: string | null }) => Promise<Record<string, never>>;
  configRead?: () => Promise<{ path: string; content: string; exists: boolean }>;
  configWrite?: (content: string) => Promise<{ path: string }>;
  configPath?: () => Promise<string>;
  onConfigChanged?: (handler: () => void) => () => void;
  vec0Status?: () => Promise<Vec0Status>;
  vec0Download?: () => Promise<Vec0Status>;
  voiceCheck?: () => Promise<ElectronVoiceCheckResult>;
  voiceLaunch?: () => Promise<ElectronVoiceLaunchResult>;
  permissionTryAutoDecide?: (payload: unknown) => Promise<{
    decision: 'allow' | 'reject' | null;
    event: import('./lib/permission-ledger/ledger').LastAutoPermissionDecision | null;
    token?: string | null;
  }>;
  permissionResolveDecision?: (payload: {
    token: string;
    optionId: string;
    scope?: import('./lib/permission-ledger/ledger').PermissionScope;
  }) => Promise<void>;
  permissionListDecisions?: () => Promise<import('./lib/permission-ledger/ledger').PersistedPermissionDecision[]>;
  permissionListAuditEvents?: () => Promise<import('./lib/permission-ledger/ledger').PermissionAuditEvent[]>;
  permissionRevokeDecision?: (id: number) => Promise<void>;
  permissionResetConfirmPrepare?: () => Promise<string>;
  permissionResetDecisions?: (token: string) => Promise<void>;
  terminal?: {
    create?: (params: {
      sessionId?: string;
      command: string;
      args?: string[];
      env?: Array<{ name: string; value: string }>;
      cwd?: string | null;
      outputByteLimit?: number | null;
    }) => Promise<{ terminalId: string; outputByteLimit: number }>;
    getOutput?: (params: { sessionId?: string; terminalId: string }) => Promise<{
      output: string;
      truncated: boolean;
      exitStatus?: { exitCode?: number | null; signal?: string | null } | null;
    }>;
    waitForExit?: (params: { sessionId?: string; terminalId: string }) => Promise<{
      exitCode?: number | null;
      signal?: string | null;
    }>;
    kill?: (params: { sessionId?: string; terminalId: string }) => Promise<Record<string, never>>;
    release?: (params: { sessionId?: string; terminalId: string }) => Promise<Record<string, never>>;
    onOutput?: (handler: (terminalId: string, chunk: string) => void) => () => void;
  };
};

type AppApi = {
  quit: () => Promise<void>;
};

type Vec0DownloadProgress = {
  current: number;
  total: number;
  filename?: string;
};

type Vec0Status = {
  platform: string;
  arch: string;
  supported: boolean;
  installed: boolean;
  fileSizeBytes: number;
  assetPattern: string | null;
  targetPath: string | null;
  downloadProgress: Vec0DownloadProgress | null;
};

/**
 * Two parallel preload bridges — T-302.40 / T-302.38.
 * `api` is the narrow non-optional contract; add NEW IPC here (extend AppApi).
 * `electronAPI` is the legacy broad surface; existing callers stay as-is
 * pending a future full-migration sprint (Option B, deferred from T-302.40).
 */
interface Window {
  api: AppApi;
  electronAPI?: ElectronApi;
}

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, any>;
  export default component;
}

declare module 'cytoscape-cose-bilkent' {
  const ext: cytoscape.Ext;
  export default ext;
}
