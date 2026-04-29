export interface GateTask {
  taskId: string;
  canonicalTaskId: string | null;
  lineageGateTaskId?: string | null;
  lineagePipelineRunId?: string | null;
  lineageThinkingSessionId?: string | null;
  lineageMemoryNoteId?: string | null;
  lineage_gate_task_id?: string | null;
  lineage_pipeline_run_id?: string | null;
  lineage_thinking_session_id?: string | null;
  lineage_memory_note_id?: string | null;
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

export interface GateCreateArgs extends Record<string, unknown> {
  root: string;
  taskId: string;
  title: string;
  pipeline: string;
  body?: string;
  priority?: string;
  sprint?: number;
  parent?: string;
}

export interface GateCreateResult {
  taskId: string;
  path: string;
  pipeline: string;
  sprint: number;
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

export interface GateTaskBody {
  taskId: string;
  path: string;
  markdown: string;
}

export const GATE_POLL_INTERVAL_MS = 5000;

export function createGateSnapshot(root: string, tasks: GateTask[]): GateSnapshot {
  return {
    dbPath: `${root}/.gate/gate.db`,
    exists: true,
    tasks,
  };
}
