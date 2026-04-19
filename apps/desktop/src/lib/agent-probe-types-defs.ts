import type {
  AcpMetadata,
  AgentInfo,
  ModelInfo,
  SessionConfigSelectOptions,
  SessionMode,
} from './types';

export type AgentProbeStyle = 'modern' | 'unknown';
export type AgentProbeState = 'ready' | 'offline' | 'error';

export interface AgentProbePromptCapabilities {
  image: boolean;
  audio: boolean;
  embeddedContext: boolean;
}

export interface AgentProbeSessionCapabilities {
  list: boolean;
  loadSession: boolean;
}

export interface AgentProbeTransportCapabilities {
  promptCapabilities: AgentProbePromptCapabilities;
  sessionCapabilities: AgentProbeSessionCapabilities;
}

export type AgentProbeConfigValueType =
  | 'string'
  | 'boolean'
  | 'number'
  | 'json'
  | 'unknown';

export interface AgentProbeConfigOption {
  id: string;
  name: string;
  description?: string;
  category?: string;
  type?: string;
  currentValue?: string;
  rawCurrentValue?: unknown;
  valueType?: AgentProbeConfigValueType;
  options: SessionConfigSelectOptions;
  _meta?: AcpMetadata;
}

export interface AgentProbeResult {
  style: AgentProbeStyle;
  state: AgentProbeState;
  offline: boolean;
  modes: string[];
  models: string[];
  modeOptions: SessionMode[];
  modelOptions: ModelInfo[];
  configOptions: Record<string, AgentProbeConfigOption>;
  configOptionList: AgentProbeConfigOption[];
  modeConfigOption?: AgentProbeConfigOption;
  modelConfigOption?: AgentProbeConfigOption;
  thoughtLevelConfigOption?: AgentProbeConfigOption;
  agentInfo?: AgentInfo | null;
  agentCapabilities?: AgentProbeTransportCapabilities | null;
  probedAt: number;
  error?: string;
  refreshing?: boolean;
  degradedMessage?: string;
}
