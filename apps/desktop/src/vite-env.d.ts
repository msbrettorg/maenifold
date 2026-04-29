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
  readTextFile?: (params: { path: string; line?: number; limit?: number }) => Promise<{ content: string }>;
  writeTextFile?: (params: { path: string; content: string }) => Promise<Record<string, never>>;
  gateWriteTaskFile?: (payload: { root: string; filename: string; content: string }) => Promise<{ path: string }>;
  checkEnvVarPresence?: (names: string[]) => Promise<Record<string, boolean>>;
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

interface Window {
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
