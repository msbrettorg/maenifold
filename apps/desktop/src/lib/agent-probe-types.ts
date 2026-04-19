// Re-export shim for backward compatibility.
// Types live in agent-probe-types-defs.ts; normalizers in agent-probe-normalizers.ts.
export type {
  AgentProbeStyle,
  AgentProbeState,
  AgentProbePromptCapabilities,
  AgentProbeSessionCapabilities,
  AgentProbeTransportCapabilities,
  AgentProbeConfigValueType,
  AgentProbeConfigOption,
  AgentProbeResult,
} from './agent-probe-types-defs';

export {
  isOfflineProbeError,
  normalizeAgentProbeResult,
} from './agent-probe-normalizers';
