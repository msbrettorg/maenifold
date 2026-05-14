import { describe, expect, it } from 'vitest';
import { stepDurationLabel } from '../../lib/workflow-step-duration';
import type { useWorkflowStore } from '../workflow';

type WorkflowStore = ReturnType<typeof useWorkflowStore>;
type WorkflowSessionDetail = NonNullable<WorkflowStore['selectedSession']>;

describe('WorkflowSessionDetail', () => {
  it('accepts workflow queue index fields used for step duration lookup', () => {
    const session: WorkflowSessionDetail = {
      id: 'session-1',
      uri: 'workflow://session/session-1',
      status: 'active',
      queue: ['foo', 'bar'],
      workflow_id: 'bar',
      current_workflow_index: 1,
      current_step: 1,
      total_steps: 2,
      started_at: '2026-05-14T00:00:00Z',
      updated_at: '2026-05-14T00:00:02Z',
      step_timings: {
        '2:1': {
          started_at: '2026-05-14T00:00:00Z',
          ended_at: '2026-05-14T00:00:02Z',
        },
      },
      history: [],
    };

    expect(stepDurationLabel(
      session.step_timings,
      1,
      session.current_workflow_index,
    )).toBe('Duration: 2s');
  });
});
