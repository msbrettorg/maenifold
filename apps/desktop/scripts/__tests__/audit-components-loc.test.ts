import { describe, expect, it } from 'vitest';

import { countLinesForMaxLoc } from '../audit-loc.mjs';

describe('countLinesForMaxLoc', () => {
  it.each([
    ['300-line file with trailing newline', `${'x\n'.repeat(300)}`, 300],
    ['300-line file without trailing newline', `${'x\n'.repeat(299)}x`, 299],
    ['empty file', '', 0],
    ['single line without newline', 'x', 0],
    ['single line with newline', 'x\n', 1],
  ])('matches wc -l semantics for %s', (_name, source, expected) => {
    expect(countLinesForMaxLoc(source)).toBe(expected);
  });
});
