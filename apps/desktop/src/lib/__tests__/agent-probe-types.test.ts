import { describe, expect, it } from 'vitest';
import { isOfflineProbeError, normalizeAgentProbeResult } from '../agent-probe-types';

describe('agent probe error classification', () => {
  it('classifies connection and spawn failures as offline', () => {
    expect(isOfflineProbeError('Channel closed')).toBe(true);
    expect(isOfflineProbeError('spawn failed: ENOENT')).toBe(true);
    expect(isOfflineProbeError('connection refused')).toBe(true);
    expect(isOfflineProbeError('broken pipe')).toBe(true);
    expect(isOfflineProbeError('GetConfig probe timed out after 10s')).toBe(true);
  });

  it('keeps malformed payload errors in the error state', () => {
    const probe = normalizeAgentProbeResult('invalid-payload', {
      error: 'Malformed GetConfig payload',
    });

    expect(probe.state).toBe('error');
    expect(probe.offline).toBe(false);
  });
});

describe('normalizeAgentProbeResult', () => {
  it('preserves modern style for modern probe payloads', () => {
    const probe = normalizeAgentProbeResult({
      style: 'modern',
      configOptions: [],
    });

    expect(probe.style).toBe('modern');
  });

  it('does not preserve legacy style once legacy probe support is removed', () => {
    const probe = normalizeAgentProbeResult({ style: 'legacy' });

    expect(probe.style).toBe('unknown');
  });

  it('treats config-option payloads as modern even without an explicit style', () => {
    const probe = normalizeAgentProbeResult({
      configOptions: [
        {
          id: 'mode',
          name: 'Mode',
          category: 'mode',
          currentValue: 'prompt',
          options: [{ value: 'prompt', name: 'Prompt' }],
        },
      ],
    });

    expect(probe.style).toBe('modern');
    expect(probe.modes).toEqual(['prompt']);
  });
});
