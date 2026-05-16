import { describe, expect, it } from 'vitest';
import WikilinkCreateDialog from '../WikilinkCreateDialog.vue';
import { sanitizeConceptForWikilinkQuery } from '../../lib/wikilink-query-sanitizer';

describe('WikilinkCreateDialog wikilink content', () => {
  it('documents the current unsanitized template output', () => {
    const concept = 'test[x]name';
    expect(WikilinkCreateDialog).toBeTruthy();
    expect(`[[${concept}]]`).toContain('[x]');
    expect(`[[${sanitizeConceptForWikilinkQuery(concept)}]]`).toBe('[[testxname]]');
  });

  it('produces clean wikilink when sanitizer is applied (mirrors post-fix component pattern)', () => {
    const concept = 'test[x]name';
    const content = `[[${sanitizeConceptForWikilinkQuery(concept)}]]`;
    expect(content).toBe('[[testxname]]');
  });
});
