export interface AgentProbeResult {
  modes: string[];
  models: string[];
  configOptions: Record<string, unknown>;
  probedAt: number;
  error?: string;
}
