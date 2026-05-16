import { describe, expect, it } from 'vitest';
import { normalizeWikiLinkInput } from '../useMemoryPromotion-helpers';

describe('normalizeWikiLinkInput sanitization gap', () => {
  it('removes single brackets from explicit wikilinks', () => {
    const [link] = normalizeWikiLinkInput('[[test[bracket]concept]]');
    const body = link.slice(2, -2);
    expect(body).not.toMatch(/[\[\]]/);
  });

  it('does not emit literal newlines in wikilink bodies', () => {
    const links = normalizeWikiLinkInput('concept\nwith\nnewlines');
    expect(links.every((link) => !link.slice(2, -2).includes('\n'))).toBe(true);
  });

  it('preserves pipe characters', () => {
    expect(normalizeWikiLinkInput('concept|with|pipes')).toEqual(['[[concept|with|pipes]]']);
  });
});
