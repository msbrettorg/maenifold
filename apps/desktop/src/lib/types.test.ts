import { describe, expect, it } from 'vitest';
import type { SessionSource } from './types';

describe('SessionSource', () => {
  it('SessionSource includes both agents and sampling', () => {
    const agents: SessionSource = 'agents';
    const sampling: SessionSource = 'sampling';

    expect(agents).toBe('agents');
    expect(sampling).toBe('sampling');
  });
});
