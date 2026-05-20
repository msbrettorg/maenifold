export type McpServerEntry =
  | {
      command: string;
      args?: string[];
      env?: Record<string, string>;
      timeout?: number;
      gateway?: boolean;
    }
  | {
      url: string;
      gateway?: boolean;
    };

export type McpServersMap = Record<string, McpServerEntry>;

export interface McpDiscoveredTool {
  name: string;
  description?: string | null;
  inputSchema?: unknown;
  appUi?: McpAppUiMetadata | null;
}

export interface McpAppUiMetadata {
  resourceUri: string;
  originatingServer?: string;
  csp?: string;
  permissions?: string[];
}

export type ServerHealthStatus = 'ready' | 'auth_required' | 'stale' | 'unavailable';

export interface McpServerDiscoveryResult {
  server: string;
  type: string;
  status: 'ok' | 'error';
  error?: string | null;
  tools: McpDiscoveredTool[];
}

export interface AgentConfig {
  command: string;
  env?: Record<string, string>;
  mcpServers?: McpServersMap;
}

export interface AgentsConfig {
  mcpServers?: McpServersMap;
  agents: Record<string, AgentConfig>;
  settings?: MaenifoldSettings;
}

export interface DecaySettings {
  graceDaysSequential: number;
  graceDaysWorkflows: number;
  graceDaysDefault: number;
  halfLifeDays: number;
  function: string;
}

export interface CommunitySettings {
  louvainGamma: number;
  minSharedNeighbors: number;
  minOverlap: number;
  maxSiblings: number;
  debounceMs: number;
  nmiThreshold: number;
  recomputeFileThreshold: number | null;
}

export interface VectorSettings {
  embeddingLogs: boolean;
  vectorLogs: boolean;
  disableVector: boolean;
}

export interface DebugSettings {
  trace: boolean;
  pipelineDriver: string | null;
}

export interface MaenifoldSettings {
  logRotation: string;
  debounceMs: number;
  autoSync: boolean;
  syncLogging: boolean;
  sessionCleanup: boolean;
  sqliteBusyTimeoutMs: number;
  syncLeaseMs: number;
  watcherBufferSize: number;
  incrementalOptimizeEvery: number;
  incrementalVacuumMinutes: number;
  sessionAbandonMinutes: number | null;
  snippetLength: number | null;
  syncRole: string | null;
  decay: DecaySettings;
  community: CommunitySettings;
  vector: VectorSettings;
  debug: DebugSettings;
}
