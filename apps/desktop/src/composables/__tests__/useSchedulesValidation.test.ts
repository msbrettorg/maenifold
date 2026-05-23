import { describe, expect, it } from 'vitest';
import { parseAllowedTools, parseMcpParameters } from '@/composables/schedules/useSchedulesValidation';

describe('useSchedulesValidation helpers', () => {
  it('parseAllowedTools splits comma- and newline-delimited values', () => {
    expect(parseAllowedTools('ReadMemory, SearchMemories\n ListMemories ,,')).toEqual([
      'ReadMemory',
      'SearchMemories',
      'ListMemories',
    ]);
  });

  it('parseMcpParameters accepts only JSON objects', () => {
    expect(parseMcpParameters('{"root":"/workspace"}')).toEqual({ ok: true, value: { root: '/workspace' } });
    expect(parseMcpParameters('[1,2,3]')).toEqual({ ok: false });
  });
});
