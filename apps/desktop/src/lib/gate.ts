export interface GateTask {
  taskId: string;
  canonicalTaskId: string | null;
  title: string;
  pipeline: string | null;
  state: string;
  stages: string[];
  attempts: number;
  location: string;
  path: string;
  issue: string;
}

export interface GateSnapshot {
  dbPath: string;
  exists: boolean;
  tasks: GateTask[];
}

export interface GateStatusArgs extends Record<string, unknown> {
  root?: string;
  pipeline?: string;
  state?: string;
  ready?: boolean;
  includeDone?: boolean;
}

export interface GateSyncArgs extends Record<string, unknown> {
  root?: string;
  pipeline?: string;
  rebuild?: boolean;
}

export type GateSyncResult = Record<string, unknown>;
export interface GateTaskHistoryEntry {
  stage: string;
  event: string;
  agent: string | null;
  model: string | null;
  notes: string | null;
  timestamp: string | null;
}

export const GATE_POLL_INTERVAL_MS = 5000;

export function createGateSnapshot(root: string, tasks: GateTask[]): GateSnapshot {
  return {
    dbPath: `${root}/.gate/gate.db`,
    exists: true,
    tasks,
  };
}
