import { describe, expect, it } from 'vitest';
import {
  WIKILINK_MALFORMED_MESSAGE,
  WIKILINK_REQUIRED_MESSAGE,
  containsMalformedWikiLinks,
  extractValidWikiLinks,
  hasValidWikiLink,
  wikiLinkGate,
} from './wikiLinkGate';

describe('extractValidWikiLinks', () => {
  it('extracts a single valid link', () => {
    expect(extractValidWikiLinks('see [[Alpha]] note')).toEqual(['Alpha']);
  });

  it('extracts multiple links and trims whitespace', () => {
    expect(extractValidWikiLinks('[[ Alpha ]] and [[Beta]]')).toEqual(['Alpha', 'Beta']);
  });

  it('rejects triple-open nested brackets', () => {
    expect(extractValidWikiLinks('[[[Alpha]]')).toEqual([]);
  });

  it('rejects triple-close nested brackets', () => {
    expect(extractValidWikiLinks('[[Alpha]]]')).toEqual([]);
  });

  it('rejects a concept name containing a bracket', () => {
    expect(extractValidWikiLinks('[[Al[pha]]')).toEqual([]);
  });

  it('ignores links inside a fenced code block', () => {
    const content = 'before\n```\n[[Alpha]]\n```\nafter';
    expect(extractValidWikiLinks(content)).toEqual([]);
  });

  it('ignores links inside an inline code span', () => {
    expect(extractValidWikiLinks('use `[[Alpha]]` literally')).toEqual([]);
  });

  it('still finds a real link outside code spans', () => {
    const content = '`[[Ignored]]` but [[Real]] counts';
    expect(extractValidWikiLinks(content)).toEqual(['Real']);
  });

  it('rejects an empty concept', () => {
    expect(extractValidWikiLinks('[[]] empty')).toEqual([]);
  });
});

describe('hasValidWikiLink', () => {
  it('is true with a valid link', () => {
    expect(hasValidWikiLink('[[Alpha]]')).toBe(true);
  });

  it('is false with no link', () => {
    expect(hasValidWikiLink('plain text')).toBe(false);
  });

  it('is false for whitespace-only content', () => {
    expect(hasValidWikiLink('   \n  ')).toBe(false);
  });
});

describe('wikiLinkGate', () => {
  it('passes content with a valid link', () => {
    const result = wikiLinkGate('content [[Alpha]]');
    expect(result.ok).toBe(true);
    expect(result.message).toBeNull();
    expect(result.links).toEqual(['Alpha']);
  });

  it('fails empty content with the backend-parity message', () => {
    const result = wikiLinkGate('');
    expect(result.ok).toBe(false);
    expect(result.message).toBe(WIKILINK_REQUIRED_MESSAGE);
    expect(result.links).toEqual([]);
  });

  it('fails content whose only link is in code', () => {
    const result = wikiLinkGate('```\n[[Alpha]]\n```');
    expect(result.ok).toBe(false);
    expect(result.message).toBe(WIKILINK_REQUIRED_MESSAGE);
  });

  it('fails when a valid link coexists with a malformed triple-bracket link', () => {
    const result = wikiLinkGate('[[good]] and [[[bad]]]');
    expect(result.ok).toBe(false);
    expect(result.message).toBe(WIKILINK_MALFORMED_MESSAGE);
  });

  it('passes a single well-formed link', () => {
    const result = wikiLinkGate('[[good]]');
    expect(result.ok).toBe(true);
    expect(result.message).toBeNull();
    expect(result.links).toEqual(['good']);
  });

  it('fails a nested-bracket malformed link', () => {
    const result = wikiLinkGate('[[a[b]]');
    expect(result.ok).toBe(false);
    expect(result.message).toBe(WIKILINK_MALFORMED_MESSAGE);
  });

  it('fails an unclosed wikilink opener', () => {
    const result = wikiLinkGate('[[oops');
    expect(result.ok).toBe(false);
    expect(result.message).toBe(WIKILINK_MALFORMED_MESSAGE);
  });
});

describe('containsMalformedWikiLinks (backend port)', () => {
  it('flags a triple-bracket close', () => {
    expect(containsMalformedWikiLinks('[[good]] and [[[bad]]]')).toBe(true);
  });

  it('does not flag a well-formed link', () => {
    expect(containsMalformedWikiLinks('[[good]]')).toBe(false);
  });

  it('flags a nested opening bracket', () => {
    expect(containsMalformedWikiLinks('[[a[b]]')).toBe(true);
  });

  it('flags an unclosed opener', () => {
    expect(containsMalformedWikiLinks('[[oops')).toBe(true);
  });

  it('does not flag plain text with no brackets', () => {
    expect(containsMalformedWikiLinks('plain text')).toBe(false);
  });
});
