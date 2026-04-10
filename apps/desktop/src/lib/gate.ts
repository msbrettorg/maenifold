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

interface GateWatcherEvent {
  root: string;
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
