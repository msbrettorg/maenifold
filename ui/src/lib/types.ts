// Types for the Maenifold UI application

/**
 * Transport kinds supported by the frontend.
 *
 * - `stdio`: agent runs as a local subprocess (desktop only).
 * - `websocket`: agent listens on `ws://` / `wss://` and speaks ACP over a WebSocket.
 * - `http`: agent listens on `http://` / `https://` and speaks ACP over Streamable HTTP / SSE.
 */
export type AgentTransportKind = 'stdio' | 'websocket' | 'http';

export interface AgentConfig {
  /**
   * Transport kind. Optional for backward compatibility — when omitted, the
   * config is treated as a stdio agent.
   */
  transport?: AgentTransportKind;

  // ----- stdio fields (optional when transport != 'stdio') -----
  command?: string;
  args?: string[];
  env?: Record<string, string>;

  // ----- remote fields (used when transport != 'stdio') -----
  url?: string;
  headers?: Record<string, string>;
}

export interface AgentsConfig {
  agents: Record<string, AgentConfig>;
  /**
   * Optional UI settings the daemon owns. The Tauri loader passes the
   * whole settings object through; the UI only reads
   * `settings.defaultAgent` today.
   */
  settings?: MaenifoldSettings;
}

/**
 * Subset of `MaenifoldSettings` (from `crates/maenifold-core::acp::config::settings`)
 * the UI cares about. Defined as `Record<string, unknown>` for the bag of
 * daemon-owned fields the UI doesn't touch, with explicit typing for the
 * fields the UI actually reads.
 */
export interface MaenifoldSettings {
  /**
   * Preferred default agent name for the picker. When present and the
   * named agent exists in `agents`, AgentSelector auto-selects it. When
   * unset or pointing at an unknown agent, the picker falls back to the
   * first agent. Edit this in `~/maenifold/maenifold.json` under
   * `settings.defaultAgent`.
   */
  defaultAgent?: string;
  // Other daemon-owned settings tunnelled through unchanged.
  [key: string]: unknown;
}

/** Returns the effective transport kind for an agent config. */
export function getTransportKind(config: AgentConfig): AgentTransportKind {
  return config.transport ?? 'stdio';
}

/** Type guard: true for legacy / explicit stdio agents. */
export function isStdioConfig(
  config: AgentConfig
): config is AgentConfig & { command: string } {
  return getTransportKind(config) === 'stdio';
}

/** Type guard: true for websocket / http agents with a non-empty URL. */
export function isRemoteConfig(
  config: AgentConfig
): config is AgentConfig & { url: string } {
  const kind = getTransportKind(config);
  return (
    (kind === 'websocket' || kind === 'http') &&
    typeof config.url === 'string' &&
    config.url.length > 0
  );
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

// Slash Commands
export interface SlashCommand {
  name: string;
  description: string;
  hint?: string;
}

// Session Config Options — preferred ACP surface, replaces deprecated
// session-modes and session-models. See:
//   https://agentclientprotocol.com/protocol/session-config-options
//
// Mirrors @agentclientprotocol/sdk's `SessionConfigOption`, but kept
// independent so the Vue layer doesn't import deep schema types directly.

/** Semantic category — reserved names per spec, plus `_…` for custom. */
export type ConfigOptionCategory = 'mode' | 'model' | 'thought_level' | string;

export interface ConfigSelectOption {
  value: string;
  name: string;
  description?: string;
}

export interface ConfigSelectGroup {
  group: string;
  name: string;
  options: ConfigSelectOption[];
}

/**
 * Wire shape from the agent: either a flat list of options or grouped lists.
 * Helpers should always normalize via `flattenSelectOptions`.
 */
export type ConfigSelectOptions = ConfigSelectOption[] | ConfigSelectGroup[];

export interface ConfigOptionBase {
  id: string;
  name: string;
  description?: string;
  category?: ConfigOptionCategory;
}

export interface ConfigOptionSelect extends ConfigOptionBase {
  type: 'select';
  currentValue: string;
  options: ConfigSelectOptions;
}

export interface ConfigOptionBoolean extends ConfigOptionBase {
  type: 'boolean';
  currentValue: boolean;
}

export type SessionConfigOption = ConfigOptionSelect | ConfigOptionBoolean;

/** Set-option request value: select takes a string id, boolean takes a bool. */
export type SessionConfigValue =
  | { type: 'select'; value: string }
  | { type: 'boolean'; value: boolean };

/** Flatten grouped select options for renderers that don't care about groups. */
export function flattenSelectOptions(opts: ConfigSelectOptions): ConfigSelectOption[] {
  if (opts.length === 0) return [];
  // ConfigSelectGroup has `.group`; ConfigSelectOption does not.
  if ('group' in opts[0]) {
    return (opts as ConfigSelectGroup[]).flatMap((g) => g.options);
  }
  return opts as ConfigSelectOption[];
}
