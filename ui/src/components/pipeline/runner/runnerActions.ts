// Pipeline runner — action-bar enablement + cancel semantics (pure, testable).
//
// Two responsibilities, both derived purely from the current `PipelineView`:
//  1. `actionFlags` — which lifecycle buttons (run/pause/resume/cancel/delete)
//     are enabled for the pipeline's current status.
//  2. `shouldCancelRemote` — the cancel split. Cancel hits the backend
//     (`store.cancelPipeline`) ONLY when the pipeline is actually executing
//     remotely; otherwise the runner does a purely local cleanup (stop polling,
//     clear run state) without a backend round-trip. This mirrors the old UI's
//     `pipeline-lifecycle-actions.ts:80-103` intent.

import type { PipelineStatus, PipelineView } from '../../../lib/pipeline-types';

const TERMINAL_STATUSES: ReadonlySet<PipelineStatus> = new Set<PipelineStatus>([
  'completed',
  'abandoned',
  'cancelled',
  'stuck',
  'failed',
]);

export function isTerminal(status: PipelineStatus): boolean {
  return TERMINAL_STATUSES.has(status);
}

// A pipeline is "remote-running" — i.e. has live work the backend must be told
// to cancel — when the runtime `running` flag is set, or its status is `running`
// or `active`. The old UI treated `active` as remote-cancellable (a dispatched
// but momentarily idle pipeline still has backend run state to tear down).
export function shouldCancelRemote(view: PipelineView | null): boolean {
  if (!view) return false;
  return view.running === true || view.status === 'running' || view.status === 'active';
}

export interface ActionFlags {
  canRun: boolean;
  canPause: boolean;
  canResume: boolean;
  canCancel: boolean;
  canDelete: boolean;
}

// Generation/dispose gating for the poll loop. A poll captures the target id +
// a generation token at START; when the non-mutating fetch resolves we apply the
// result ONLY when it is still current — not disposed (unmounted), same
// generation (no intervening pipeline switch / restart), and the live selection
// still points at the polled id. This is what stops an in-flight poll for
// pipeline A from clobbering selection/logs after the user switches to B.
export interface PollGate {
  capturedId: string;
  capturedGeneration: number;
  currentId: string | null;
  currentGeneration: number;
  disposed: boolean;
}

export function isPollCurrent(gate: PollGate): boolean {
  if (gate.disposed) return false;
  if (gate.capturedGeneration !== gate.currentGeneration) return false;
  return gate.currentId === gate.capturedId;
}

const DISABLED: ActionFlags = {
  canRun: false,
  canPause: false,
  canResume: false,
  canCancel: false,
  canDelete: false,
};

// Derive button enablement from the selected pipeline's status.
//  - run:    a non-terminal, non-paused, non-running pipeline can be (re)started.
//  - pause:  only a remote-running pipeline can be paused.
//  - resume: only a paused pipeline can be resumed.
//  - cancel: any non-terminal pipeline can be cancelled (remote or local).
//  - delete: always available once a pipeline is selected (force path recovers
//            completed/unreadable ones).
export function actionFlags(view: PipelineView | null): ActionFlags {
  if (!view) return { ...DISABLED };
  const status = view.status;
  const terminal = isTerminal(status);
  const remoteRunning = shouldCancelRemote(view);

  return {
    canRun: !terminal && status !== 'paused' && !remoteRunning,
    canPause: remoteRunning && status !== 'paused',
    canResume: status === 'paused',
    canCancel: !terminal,
    canDelete: true,
  };
}
