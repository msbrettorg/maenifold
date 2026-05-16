import { describe, expect, it } from 'vitest';
import { sanitizeConceptForWikilinkQuery } from '../wikilink-query-sanitizer';

describe('sanitizeConceptForWikilinkQuery', () => {
  it('strips square brackets from concept names', () => {
    expect(sanitizeConceptForWikilinkQuery(' test[bracket]name ')).toBe('testbracketname');
  });

  it('replaces CRLF with spaces', () => {
    expect(sanitizeConceptForWikilinkQuery('concept\r\nwith\r\nnewlines')).toBe('concept with newlines');
  });

  it('preserves pipe characters', () => {
    expect(sanitizeConceptForWikilinkQuery('concept|with|pipes')).toBe('concept|with|pipes');
  });

  it('trims surrounding whitespace', () => {
    expect(sanitizeConceptForWikilinkQuery('  concept  ')).toBe('concept');
  });
});
