import { beforeEach, describe, expect, it } from 'vitest';
import {
  __resetAgentProbeCacheForTests,
  loadAgentProbeCache,
  persistAgentProbeCache,
} from '../agent-probe-cache';
import { normalizeAgentProbeResult } from '../agent-probe-normalizers';

function writeReadyProbe(agentName: string) {
  persistAgentProbeCache(new Map([
    [agentName, normalizeAgentProbeResult({ configOptions: [] })],
  ]));
}

describe('agent probe cache reset helper', () => {
  beforeEach(() => {
    __resetAgentProbeCacheForTests();
    window.localStorage.clear();
  });

  it('1. writes and loads a ready probe entry', () => {
    writeReadyProbe('alpha');

    expect(loadAgentProbeCache().get('alpha')).toEqual(
      expect.objectContaining({ state: 'ready' }),
    );
  });

  it('2. keeps the stale in-memory singleton after localStorage.clear() alone', () => {
    writeReadyProbe('alpha');

    window.localStorage.clear();

    expect(loadAgentProbeCache().get('alpha')).toEqual(
      expect.objectContaining({ state: 'ready' }),
    );
  });

  it('3. proves localStorage.clear() alone does not remove the stale entry', () => {
    writeReadyProbe('alpha');
    expect(loadAgentProbeCache().get('alpha')).toEqual(
      expect.objectContaining({ state: 'ready' }),
    );

    window.localStorage.clear();

    expect(loadAgentProbeCache().get('alpha')).toEqual(
      expect.objectContaining({ state: 'ready' }),
    );
  });

  it('4. removes the stale in-memory singleton only after reset', () => {
    writeReadyProbe('alpha');
    expect(loadAgentProbeCache().get('alpha')).toEqual(
      expect.objectContaining({ state: 'ready' }),
    );

    window.localStorage.clear();
    expect(loadAgentProbeCache().get('alpha')).toEqual(
      expect.objectContaining({ state: 'ready' }),
    );

    __resetAgentProbeCacheForTests();

    expect(loadAgentProbeCache().get('alpha')).toBeUndefined();
  });

  it('5. allows fresh writes after reset', () => {
    writeReadyProbe('alpha');

    __resetAgentProbeCacheForTests();
    writeReadyProbe('beta');

    expect(loadAgentProbeCache().get('beta')).toEqual(
      expect.objectContaining({ state: 'ready' }),
    );
  });
});
