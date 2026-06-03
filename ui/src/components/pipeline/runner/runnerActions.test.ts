import { describe, expect, it } from 'vitest';
import { actionFlags, isPollCurrent, isTerminal, shouldCancelRemote } from './runnerActions';
import type { PipelineStatus, PipelineView } from '../../../lib/pipeline-types';

function view(partial: Partial<PipelineView> & { status: PipelineStatus }): PipelineView {
  const base: PipelineView = {
    pipelineId: 'pid',
    name: 'demo',
    status: 'active',
    nodes: [],
    edges: [],
    ready: [],
    active: [],
    completed: [],
    mermaid: '',
    running: false,
  };
  return { ...base, ...partial };
}

describe('runnerActions.isTerminal', () => {
  it('matches the terminal set', () => {
    for (const s of ['completed', 'abandoned', 'cancelled', 'stuck', 'failed'] as PipelineStatus[]) {
      expect(isTerminal(s)).toBe(true);
    }
    for (const s of ['pending', 'active', 'running', 'paused', 'retrying'] as PipelineStatus[]) {
      expect(isTerminal(s)).toBe(false);
    }
  });
});

describe('runnerActions.shouldCancelRemote (cancel split)', () => {
  it('is true when running flag is set', () => {
    expect(shouldCancelRemote(view({ status: 'pending', running: true }))).toBe(true);
  });

  it('is true when status is running or active', () => {
    expect(shouldCancelRemote(view({ status: 'running' }))).toBe(true);
    expect(shouldCancelRemote(view({ status: 'active' }))).toBe(true);
  });

  it('is false for an idle/pending/paused/terminal pipeline with running=false', () => {
    expect(shouldCancelRemote(view({ status: 'pending' }))).toBe(false);
    expect(shouldCancelRemote(view({ status: 'paused' }))).toBe(false);
    expect(shouldCancelRemote(view({ status: 'completed' }))).toBe(false);
  });

  it('is false for a null view', () => {
    expect(shouldCancelRemote(null)).toBe(false);
  });
});

describe('runnerActions.actionFlags', () => {
  it('disables everything for a null view', () => {
    expect(actionFlags(null)).toEqual({
      canRun: false,
      canPause: false,
      canResume: false,
      canCancel: false,
      canDelete: false,
    });
  });

  it('pending: can run, can cancel, can delete; cannot pause/resume', () => {
    expect(actionFlags(view({ status: 'pending' }))).toEqual({
      canRun: true,
      canPause: false,
      canResume: false,
      canCancel: true,
      canDelete: true,
    });
  });

  it('running: can pause + cancel, cannot run/resume', () => {
    const flags = actionFlags(view({ status: 'running', running: true }));
    expect(flags.canRun).toBe(false);
    expect(flags.canPause).toBe(true);
    expect(flags.canResume).toBe(false);
    expect(flags.canCancel).toBe(true);
  });

  it('paused: can resume + cancel, cannot run/pause', () => {
    const flags = actionFlags(view({ status: 'paused' }));
    expect(flags.canRun).toBe(false);
    expect(flags.canPause).toBe(false);
    expect(flags.canResume).toBe(true);
    expect(flags.canCancel).toBe(true);
  });

  it('terminal (completed): only delete is enabled', () => {
    expect(actionFlags(view({ status: 'completed' }))).toEqual({
      canRun: false,
      canPause: false,
      canResume: false,
      canCancel: false,
      canDelete: true,
    });
  });
});

describe('runnerActions.isPollCurrent (poll gating)', () => {
  const base = {
    capturedId: 'A',
    capturedGeneration: 1,
    currentId: 'A',
    currentGeneration: 1,
    disposed: false,
  };

  it('is current when id + generation match and not disposed', () => {
    expect(isPollCurrent({ ...base })).toBe(true);
  });

  it('is stale when disposed (composable unmounted)', () => {
    expect(isPollCurrent({ ...base, disposed: true })).toBe(false);
  });

  it('is stale when the generation changed (switch/restart/stop)', () => {
    expect(isPollCurrent({ ...base, currentGeneration: 2 })).toBe(false);
  });

  it('is stale when the live selection moved off the polled id', () => {
    expect(isPollCurrent({ ...base, currentId: 'B' })).toBe(false);
  });

  it('is stale when the selection was cleared (currentId null)', () => {
    expect(isPollCurrent({ ...base, currentId: null })).toBe(false);
  });
});
