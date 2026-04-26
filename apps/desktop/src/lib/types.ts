import type {
  AvailableCommand as SdkAvailableCommand,
  ContentBlock as SdkContentBlock,
  PromptCapabilities as SdkPromptCapabilities,
  ToolCallContent as SdkToolCallContent,
} from '@agentclientprotocol/sdk';
import type { AppAuthMethod } from './auth-methods';

// Types for maenifold application

export type AcpMetadata = Record<string, unknown> | null;

interface AcpMetadataCarrier {
  _meta?: AcpMetadata;
}

export type McpServerEntry =
  | {
      type: 'stdio';
      name: string;
      command: string;
      args?: string[];
    }
  | {
      type: 'http';
      name: string;
      url: string;
    };

export interface McpDiscoveredTool {
  name: string;
  description?: string | null;
  inputSchema: Record<string, unknown>;
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
  mcpServers?: McpServerEntry[];
}

export interface AgentsConfig {
  mcpServers?: McpServerEntry[];
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

export interface EasterEggsSettings {
  wallpaper: WallpaperSettings;
}

export interface WallpaperSettings {
  enabled: boolean;
  mode: string;
  prompt: string;
  azure: WallpaperAzureSettings;
  size: string;
  quality: string;
  continuity: boolean;
  inputFidelity: string;
  maxRetries: number;
  communityCount: number;
  rotateMinutes: number;
}

export interface WallpaperAzureSettings {
  endpointEnvVar: string;
  apiKeyEnvVar: string;
  imageModel: string;
  smallModel: string;
  reasoningModel: string;
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
  easterEggs?: EasterEggsSettings;
}

export interface AgentInstance {
  id: string;
  name: string;
}

export interface AgentMessage {
  agent_id: string;
  message: string;
}

export interface AgentStderr {
  agent_id: string;
  line: string;
}

export interface SavedSession {
  id: string;
  agentName: string;
  sessionId: string;
  title: string;
  lastUpdated: number;
  cwd: string;
  supportsLoadSession?: boolean; // Whether the agent supports session/load
}

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  blocks?: ContentBlock[];
  thought?: string;
  timestamp: number;
  toolCalls?: ToolCallInfo[];
}

export interface ToolCallInfo extends AcpMetadataCarrier {
  toolCallId: string;
  title: string;
  kind: string;
  status: 'pending' | 'in_progress' | 'completed' | 'failed' | 'cancelled';
  appUi?: McpAppUiMetadata | null;
  locations?: { path: string; line?: number }[];
  content?: ToolCallContentBlock[];
  rawInput?: Record<string, unknown>;
  rawOutput?: Record<string, unknown>;
}

export interface PermissionRequest extends AcpMetadataCarrier {
  sessionId: string;
  toolCall: ToolCallInfo;
  options: PermissionOption[];
}

export interface PermissionOption extends AcpMetadataCarrier {
  kind: string;
  name: string;
  optionId: string;
}

// Session Modes
export interface SessionMode extends AcpMetadataCarrier {
  id: string;
  name: string;
  description?: string;
}

// Session Config Options
export interface SessionConfigSelectOption extends AcpMetadataCarrier {
  value: string;
  name: string;
  description?: string;
}

export interface SessionConfigSelectGroup extends AcpMetadataCarrier {
  group: string;
  name: string;
  options: SessionConfigSelectOption[];
}

export type SessionConfigSelectOptions =
  | SessionConfigSelectOption[]
  | SessionConfigSelectGroup[];

export type SessionConfigValue = string | boolean;

export interface SessionConfigOption extends AcpMetadataCarrier {
  id: string;
  name: string;
  description?: string;
  category?: string;
  type: 'select';
  currentValue: string;
  rawCurrentValue?: SessionConfigValue;
  valueType?: 'string' | 'boolean';
  options: SessionConfigSelectOptions;
}

// Models
export interface ModelInfo extends AcpMetadataCarrier {
  modelId: string;
  name: string;
  description?: string;
}

export interface AgentInfo extends AcpMetadataCarrier {
  name: string;
  title?: string;
  version?: string;
}

export type PromptCapabilities = SdkPromptCapabilities & AcpMetadataCarrier;

export type AvailableCommand = SdkAvailableCommand & AcpMetadataCarrier;

// The ACP SDK content block type is intentionally strict, but maenifold's UI
// expects these optional fields on resource_link blocks.
export type ResourceLinkContentBlock = Extract<SdkContentBlock, { type: 'resource_link' }> & {
  // For resource_link blocks:
  title?: string | null;
  description?: string | null;
  // ACP often encodes int64 sizes as bigint in TypeScript; allow both.
  size?: number | bigint | null;
};

export type ContentBlock =
  | Exclude<SdkContentBlock, { type: 'resource_link' }>
  | ResourceLinkContentBlock;

export type ToolCallInlineContentBlock = Extract<SdkToolCallContent, { type: 'content' }>;

export type ToolCallDiffContentBlock = Extract<SdkToolCallContent, { type: 'diff' }>;

export type ToolCallTerminalContentBlock = Extract<SdkToolCallContent, { type: 'terminal' }> & {
  output?: string;
  truncated?: boolean;
  released?: boolean;
  exitStatus?: {
    exitCode?: number | null;
    signal?: string | null;
  } | null;
};

export type ToolCallUnknownContentBlock = {
  type: string;
  [key: string]: unknown;
};

export type ToolCallContentBlock =
  | ToolCallInlineContentBlock
  | ToolCallDiffContentBlock
  | ToolCallTerminalContentBlock
  | ToolCallUnknownContentBlock;

export interface PlanEntry extends AcpMetadataCarrier {
  content: string;
  priority: 'high' | 'medium' | 'low';
  status: 'pending' | 'in_progress' | 'completed';
}

export interface SessionState {
  sessionId: string;
  /** Client-side bridge instance ID — used to route relay disconnects to the correct session. */
  agentId: string | null;
  agentName: string;
  cwd: string;
  session: SavedSession | null;
  isConnected: boolean;
  isConnecting: boolean;
  isLoading: boolean;
  error: string | null;
  messages: ChatMessage[];
  toolCalls: Map<string, ToolCallInfo>;
  pendingPermissions: PermissionRequest[];
  pendingAuthMethods: AppAuthMethod[];
  pendingAuthAgentName: string;
  agentInfo: AgentInfo | null;
  agentPromptCapabilities: PromptCapabilities | null;
  supportsSessionList: boolean;
  availableCommands: AvailableCommand[];
  configOptions: SessionConfigOption[];
  hasConfigOptions: boolean;
  usage: Record<string, unknown> | null;
  currentPlan: PlanEntry[] | null;
  startupPhase: string;
  startupLogs: string[];
  startupElapsed: number;
}

export function generateSessionId(): string {
  return `session-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
}

export function createEmptySessionState(
  sessionId: string,
  agentName: string,
  cwd: string
): SessionState {
  return {
    sessionId,
    agentId: null,
    agentName,
    cwd,
    session: null,
    isConnected: false,
    isConnecting: false,
    isLoading: false,
    error: null,
    messages: [],
    toolCalls: new Map<string, ToolCallInfo>(),
    pendingPermissions: [],
    pendingAuthMethods: [],
    pendingAuthAgentName: '',
    agentInfo: null,
    agentPromptCapabilities: null,
    supportsSessionList: false,
    availableCommands: [],
    configOptions: [],
    hasConfigOptions: false,
    usage: null,
    currentPlan: null,
    startupPhase: '',
    startupLogs: [],
    startupElapsed: 0,
  };
}
