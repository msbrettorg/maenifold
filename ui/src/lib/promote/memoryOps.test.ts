import { describe, expect, it } from 'vitest';
import {
  buildEditRequest,
  buildWriteRequest,
  classifyMemoryError,
  classifyMemoryOutcome,
} from './memoryOps';

describe('buildEditRequest', () => {
  it('builds an append request with checksum', () => {
    const request = buildEditRequest('append', 'memory://note', 'body [[X]]', 'abc123');
    expect(request).toEqual({
      identifier: 'memory://note',
      operation: 'append',
      content: 'body [[X]]',
      checksum: 'abc123',
    });
  });

  it('builds a prepend request', () => {
    const request = buildEditRequest('prepend', 'memory://note', 'top', 'cs');
    expect(request.operation).toBe('prepend');
    expect(request.findText).toBeUndefined();
    expect(request.sectionName).toBeUndefined();
  });

  it('builds a replace_section request with a trimmed section name', () => {
    const request = buildEditRequest('replace_section', 'memory://note', 'new', 'cs', {
      sectionName: '  Summary  ',
    });
    expect(request.sectionName).toBe('Summary');
    expect(request.operation).toBe('replace_section');
  });

  it('pins find_replace to expectedCount 1', () => {
    const request = buildEditRequest('find_replace', 'memory://note', 'new', 'cs', {
      findText: 'old',
    });
    expect(request.operation).toBe('find_replace');
    expect(request.findText).toBe('old');
    expect(request.expectedCount).toBe(1);
  });
});

describe('buildWriteRequest', () => {
  it('trims title and omits empty folder', () => {
    const request = buildWriteRequest('  Title  ', 'content [[X]]', '   ');
    expect(request).toEqual({ title: 'Title', content: 'content [[X]]' });
  });

  it('includes a trimmed folder when provided', () => {
    const request = buildWriteRequest('Title', 'content', ' notes ');
    expect(request.folder).toBe('notes');
  });
});

describe('classifyMemoryOutcome', () => {
  it('detects a stale checksum mismatch', () => {
    const raw = 'ERROR: Checksum mismatch. Expected: aaa, Actual: bbb';
    expect(classifyMemoryOutcome(raw)).toEqual({ kind: 'stale' });
  });

  it('detects a checksum mismatch wrapped in an envelope', () => {
    const raw = '{"success":true,"data":"ERROR: Checksum mismatch. Expected: x, Actual: y"}';
    expect(classifyMemoryOutcome(raw)).toEqual({ kind: 'stale' });
  });

  it('detects a wikilink validation error', () => {
    const raw = 'ERROR: Content must contain at least one [[concept]] in double brackets.';
    const outcome = classifyMemoryOutcome(raw);
    expect(outcome.kind).toBe('error');
    expect(outcome.message).toContain('must contain at least one');
  });

  it('detects an ERROR prefix inside an envelope', () => {
    const raw = '{"success":true,"data":"ERROR: Section not found"}';
    const outcome = classifyMemoryOutcome(raw);
    expect(outcome.kind).toBe('error');
    expect(outcome.message).toBe('Section not found');
  });

  it('treats a normal success summary as ok', () => {
    const raw = 'Wrote memory://notes/title (3 wikilinks)';
    expect(classifyMemoryOutcome(raw)).toEqual({ kind: 'ok' });
  });
});

describe('classifyMemoryError', () => {
  it('maps a thrown checksum mismatch to stale', () => {
    const outcome = classifyMemoryError(new Error('Checksum mismatch. Expected x'));
    expect(outcome).toEqual({ kind: 'stale' });
  });

  it('maps other thrown errors to error with the message', () => {
    const outcome = classifyMemoryError(new Error('boom'));
    expect(outcome.kind).toBe('error');
    expect(outcome.message).toBe('boom');
  });

  it('handles non-Error throws', () => {
    const outcome = classifyMemoryError('plain string failure');
    expect(outcome.kind).toBe('error');
    expect(outcome.message).toBe('plain string failure');
  });
});
