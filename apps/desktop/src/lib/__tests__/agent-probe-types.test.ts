import { describe, expect, it } from 'vitest';
import { isOfflineProbeError, normalizeAgentProbeResult } from '../agent-probe-types';

describe('agent probe error classification', () => {
  it('classifies connection and spawn failures as offline', () => {
    expect(isOfflineProbeError('Channel closed')).toBe(true);
    expect(isOfflineProbeError('spawn failed: ENOENT')).toBe(true);
    expect(isOfflineProbeError('connection refused')).toBe(true);
    expect(isOfflineProbeError('broken pipe')).toBe(true);
    expect(isOfflineProbeError('ListAgents probe timed out after 10s')).toBe(true);
  });

  it('keeps malformed payload errors in the error state', () => {
    const probe = normalizeAgentProbeResult('invalid-payload', {
      error: 'Malformed ListAgents payload',
    });

    expect(probe.state).toBe('error');
    expect(probe.offline).toBe(false);
  });
});
