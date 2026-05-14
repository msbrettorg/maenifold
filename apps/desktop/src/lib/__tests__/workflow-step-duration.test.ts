import { describe, expect, it } from 'vitest';
import { stepDurationLabel } from '../workflow-step-duration';
import type { WorkflowStepTimings } from '../../stores/workflow';

describe('stepDurationLabel', () => {
  it('returns key "1" for the first workflow (index 0)', () => {
    const timings: WorkflowStepTimings = {
      '1': {
        started_at: '2026-04-29T12:00:00Z',
        ended_at: '2026-04-29T12:00:01.200Z',
      },
    };

    expect(stepDurationLabel(timings, 1, 0)).toBe('Duration: 1.2s');
  });

  it('returns key "<workflowIndex+1>:<n>" for queued workflows beyond the first', () => {
    const timings: WorkflowStepTimings = {
      '2:1': {
        started_at: '2026-04-29T12:00:00Z',
        ended_at: '2026-04-29T12:00:05Z',
      },
    };

    expect(stepDurationLabel(timings, 1, 1)).toBe('Duration: 5s');
  });

  it('returns "" when the queued workflow step has no timing entry', () => {
    const timings: WorkflowStepTimings = {
      '1': {
        started_at: '2026-04-29T12:00:00Z',
        ended_at: '2026-04-29T12:00:05Z',
      },
    };

    expect(stepDurationLabel(timings, 1, 1)).toBe('');
  });
});
