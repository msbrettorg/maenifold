import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

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

export type GateTaskHistoryEntry = Record<string, unknown>;

export const GATE_POLL_INTERVAL_MS = 5000;

interface GateWatcherEvent {
  root: string;
}

export function createGateSnapshot(root: string, tasks: GateTask[]): GateSnapshot {
  return {
    dbPath: `${root}/.gate/gate.db`,
    exists: true,
    tasks,
  };
}

export function loadGateSnapshot(root: string): Promise<GateSnapshot> {
  return invoke('load_gate_snapshot', { root });
}

export function readGateTaskMarkdown(path: string): Promise<string> {
  return invoke('read_gate_task_markdown', { path });
}

export function startGateWatcher(root: string): Promise<void> {
  return invoke('start_gate_watcher', { root });
}

export function stopGateWatcher(): Promise<void> {
  return invoke('stop_gate_watcher');
}

export function onGateWatcherEvent(handler: (event: GateWatcherEvent) => void): Promise<UnlistenFn> {
  return listen<GateWatcherEvent>('gate://changed', ({ payload }) => handler(payload));
}
