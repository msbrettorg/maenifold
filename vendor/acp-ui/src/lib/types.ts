import type {
  AvailableCommand as SdkAvailableCommand,
  PlanEntry as SdkPlanEntry,
  PromptCapabilities as SdkPromptCapabilities,
} from '@agentclientprotocol/sdk';

// Types for maenifold application

export interface AgentConfig {
  command: string;
  args: string[];
  env?: Record<string, string>;
}

export interface AgentsConfig {
  agents: Record<string, AgentConfig>;
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
  thought?: string;
  timestamp: number;
  toolCalls?: ToolCallInfo[];
}

export interface ToolCallInfo {
  toolCallId: string;
  title: string;
  kind: string;
  status: 'pending' | 'in_progress' | 'completed' | 'failed';
  locations?: { path: string }[];
}

export interface PermissionRequest {
  sessionId: string;
  toolCall: ToolCallInfo;
  options: PermissionOption[];
}

export interface PermissionOption {
  kind: string;
  name: string;
  optionId: string;
}

// Session Modes
export interface SessionMode {
  id: string;
  name: string;
  description?: string;
}

export interface SessionModeState {
  currentModeId: string;
  availableModes: SessionMode[];
}

// Session Config Options
export interface SessionConfigSelectOption {
  value: string;
  name: string;
  description?: string;
}

export interface SessionConfigSelectGroup {
  group: string;
  name: string;
  options: SessionConfigSelectOption[];
}

export type SessionConfigSelectOptions =
  | SessionConfigSelectOption[]
  | SessionConfigSelectGroup[];

export interface SessionConfigOption {
  id: string;
  name: string;
  description?: string;
  category?: string;
  type: 'select';
  currentValue: string;
  options: SessionConfigSelectOptions;
}

// Slash Commands
export interface SlashCommand {
  name: string;
  description: string;
  hint?: string;
}

// Models
export interface ModelInfo {
  modelId: string;
  name: string;
  description?: string;
}

export type PromptCapabilities = SdkPromptCapabilities;

export type AvailableCommand = SdkAvailableCommand;

export type PlanEntry = SdkPlanEntry;

export interface SessionState {
  sessionId: string;
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
  pendingAuthMethods: any[];
  pendingAuthAgentName: string;
  agentInfo: { name: string; title?: string; version?: string } | null;
  agentPromptCapabilities: PromptCapabilities | null;
  supportsSessionList: boolean;
  availableModes: SessionMode[];
  currentModeId: string;
  availableModels: ModelInfo[];
  currentModelId: string;
  availableCommands: AvailableCommand[];
  configOptions: SessionConfigOption[];
  hasConfigOptions: boolean;
  currentPlan: PlanEntry[];
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
    availableModes: [],
    currentModeId: '',
    availableModels: [],
    currentModelId: '',
    availableCommands: [],
    configOptions: [],
    hasConfigOptions: false,
    currentPlan: [],
    startupPhase: '',
    startupLogs: [],
    startupElapsed: 0,
  };
}
