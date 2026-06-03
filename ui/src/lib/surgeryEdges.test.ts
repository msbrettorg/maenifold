import { describe, expect, it } from 'vitest';
import {
  DEFAULT_EDGE_TYPE,
  blankEdgeDraft,
  buildEdges,
  collisions,
  edgeDraftError,
  toPipelineEdge,
  type EdgeDraft,
} from './surgeryEdges';

describe('blankEdgeDraft / defaults', () => {
  it('defaults to the success type and carries no conditional fields', () => {
    expect(DEFAULT_EDGE_TYPE).toBe('success');
    const draft = blankEdgeDraft('a', 'b');
    expect(draft).toEqual({ source: 'a', target: 'b', type: 'success' });
  });
});

describe('edgeDraftError', () => {
  it('requires source, target, and distinct endpoints', () => {
    expect(edgeDraftError({ source: '', target: 'b', type: 'success' })).toMatch(/source/);
    expect(edgeDraftError({ source: 'a', target: '', type: 'success' })).toMatch(/target/);
    expect(edgeDraftError({ source: 'a', target: 'a', type: 'success' })).toMatch(/differ/);
  });

  it('accepts success / always with no extra fields', () => {
    expect(edgeDraftError({ source: 'a', target: 'b', type: 'success' })).toBeNull();
    expect(edgeDraftError({ source: 'a', target: 'b', type: 'always' })).toBeNull();
  });

  it('requires path AND equals for conditional edges', () => {
    expect(edgeDraftError({ source: 'a', target: 'b', type: 'conditional' })).toMatch(/path/);
    expect(
      edgeDraftError({ source: 'a', target: 'b', type: 'conditional', path: 'x' })
    ).toMatch(/equals/);
    expect(
      edgeDraftError({ source: 'a', target: 'b', type: 'conditional', path: 'x', equals: '1' })
    ).toBeNull();
  });
});

describe('toPipelineEdge', () => {
  it('returns null for an invalid draft', () => {
    expect(toPipelineEdge({ source: 'a', target: 'a', type: 'success' })).toBeNull();
  });

  it('omits conditional fields for non-conditional edges', () => {
    const edge = toPipelineEdge({ source: 'a', target: 'b', type: 'success' });
    expect(edge).toEqual({ source: 'a', target: 'b', type: 'success' });
    expect('path' in (edge ?? {})).toBe(false);
  });

  it('includes path + equals for conditional edges', () => {
    const edge = toPipelineEdge({
      source: 'a',
      target: 'b',
      type: 'conditional',
      path: ' status ',
      equals: 'done',
    });
    expect(edge).toEqual({ source: 'a', target: 'b', type: 'conditional', path: 'status', equals: 'done' });
  });
});

describe('buildEdges', () => {
  it('returns the built edges when all drafts are valid', () => {
    const drafts: EdgeDraft[] = [
      { source: 'a', target: 'b', type: 'success' },
      { source: 'b', target: 'c', type: 'always' },
    ];
    const result = buildEdges(drafts);
    expect('edges' in result).toBe(true);
    if ('edges' in result) expect(result.edges).toHaveLength(2);
  });

  it('reports the FIRST offending row with its index', () => {
    const drafts: EdgeDraft[] = [
      { source: 'a', target: 'b', type: 'success' },
      { source: 'c', target: 'c', type: 'success' },
    ];
    const result = buildEdges(drafts);
    expect('error' in result).toBe(true);
    if ('error' in result) {
      expect(result.index).toBe(1);
      expect(result.error).toMatch(/differ/);
    }
  });
});

describe('collisions', () => {
  it('returns the sorted intersection of two id sets', () => {
    expect(collisions(['a', 'b', 'c'], ['c', 'b', 'z'])).toEqual(['b', 'c']);
  });

  it('returns empty when there is no overlap', () => {
    expect(collisions(['a', 'b'], ['x', 'y'])).toEqual([]);
  });
});
