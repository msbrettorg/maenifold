import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import {
  statusKey,
  edgeConditionLabel,
  edgeMappingIndicator,
  loadPositions,
  savePositions,
  viewToFlowEdges,
  type FlowEdgeData,
} from './canvasBridge';
import type { NodeStatus, PipelineEdge } from '../../lib/pipeline-types';

// ---------------------------------------------------------------------------
// localStorage stub: a Map-backed implementation installed on globalThis for
// the persistence round-trip tests. canvasBridge reads/writes via
// `globalThis.localStorage?.{get,set}Item`, so a minimal shim is sufficient.
// ---------------------------------------------------------------------------

class LocalStorageStub {
  private store = new Map<string, string>();
  getItem(key: string): string | null {
    return this.store.has(key) ? (this.store.get(key) as string) : null;
  }
  setItem(key: string, value: string): void {
    this.store.set(key, String(value));
  }
  removeItem(key: string): void {
    this.store.delete(key);
  }
  clear(): void {
    this.store.clear();
  }
  // Test-only escape hatch to seed malformed payloads directly.
  rawSet(key: string, value: string): void {
    this.store.set(key, value);
  }
}

describe('canvasBridge.statusKey', () => {
  it('maps active and retrying to active', () => {
    expect(statusKey('active')).toBe('active');
    expect(statusKey('retrying')).toBe('active');
  });

  it('maps completed to completed', () => {
    expect(statusKey('completed')).toBe('completed');
  });

  it('maps abandoned and cancelled to abandoned', () => {
    expect(statusKey('abandoned')).toBe('abandoned');
    expect(statusKey('cancelled')).toBe('abandoned');
  });

  it('maps stuck and failed to stuck', () => {
    expect(statusKey('stuck')).toBe('stuck');
    expect(statusKey('failed')).toBe('stuck');
  });

  it('maps pending/ready/skipped to pending', () => {
    expect(statusKey('pending')).toBe('pending');
    expect(statusKey('ready')).toBe('pending');
    expect(statusKey('skipped')).toBe('pending');
  });

  it('falls back to pending for undefined / unknown status', () => {
    expect(statusKey(undefined)).toBe('pending');
    // An out-of-enum value still hits the default branch.
    expect(statusKey('bogus' as unknown as NodeStatus)).toBe('pending');
  });
});

describe('canvasBridge.edgeConditionLabel', () => {
  function data(partial: Partial<FlowEdgeData>): FlowEdgeData {
    return { edgeType: 'conditional', fanIndex: 0, fanCount: 1, ...partial };
  }

  it('returns ?{path}=<JSON.stringify(equals)> for a conditional edge with a string equals', () => {
    expect(edgeConditionLabel(data({ path: 'out.status', equals: 'done' }))).toBe(
      '?{out.status}="done"'
    );
  });

  it('reflects JSON.stringify for a numeric equals', () => {
    expect(edgeConditionLabel(data({ path: 'out.count', equals: 3 }))).toBe('?{out.count}=3');
  });

  it('reflects JSON.stringify for a boolean equals', () => {
    expect(edgeConditionLabel(data({ path: 'out.ok', equals: true }))).toBe('?{out.ok}=true');
  });

  it('returns null when edgeType is not conditional', () => {
    expect(edgeConditionLabel(data({ edgeType: 'success', path: 'out.status', equals: 'x' }))).toBeNull();
    expect(edgeConditionLabel(data({ edgeType: 'always', path: 'out.status', equals: 'x' }))).toBeNull();
  });

  it('returns null when path is absent', () => {
    expect(edgeConditionLabel(data({ equals: 'done' }))).toBeNull();
  });

  it('returns null for undefined data', () => {
    expect(edgeConditionLabel(undefined)).toBeNull();
  });
});

describe('canvasBridge.edgeMappingIndicator', () => {
  function data(partial: Partial<FlowEdgeData>): FlowEdgeData {
    return { edgeType: 'success', fanIndex: 0, fanCount: 1, ...partial };
  }

  it('returns null when no mapping present', () => {
    expect(edgeMappingIndicator(data({}))).toBeNull();
    expect(edgeMappingIndicator(undefined)).toBeNull();
  });

  it('returns warn for a declared-but-empty mapping', () => {
    expect(edgeMappingIndicator(data({ mapping: {} }))).toBe('warn');
  });

  it('returns ok for a populated mapping', () => {
    expect(edgeMappingIndicator(data({ mapping: { a: 'b' } }))).toBe('ok');
  });
});

describe('canvasBridge.loadPositions / savePositions', () => {
  let stub: LocalStorageStub;
  let original: Storage | undefined;
  const PID_A = 'pipe-A';
  const PID_B = 'pipe-B';

  beforeEach(() => {
    original = (globalThis as { localStorage?: Storage }).localStorage;
    stub = new LocalStorageStub();
    Object.defineProperty(globalThis, 'localStorage', {
      value: stub,
      configurable: true,
      writable: true,
    });
  });

  afterEach(() => {
    Object.defineProperty(globalThis, 'localStorage', {
      value: original,
      configurable: true,
      writable: true,
    });
  });

  it('round-trips a set of positions by pipelineId', () => {
    savePositions(
      [
        { id: 'n1', position: { x: 10, y: 20 } },
        { id: 'n2', position: { x: 33, y: 44 } },
      ],
      PID_A
    );
    expect(loadPositions(PID_A)).toEqual({
      n1: { x: 10, y: 20 },
      n2: { x: 33, y: 44 },
    });
  });

  it('writes to the versioned per-pipeline key', () => {
    savePositions([{ id: 'n1', position: { x: 1, y: 2 } }], PID_A);
    const raw = stub.getItem('maenifold:pipeline:positions:v1:pipe-A');
    expect(raw).not.toBeNull();
    expect(JSON.parse(raw as string)).toEqual({ n1: { x: 1, y: 2 } });
  });

  it('isolates positions across pipelineIds (per-pipeline key isolation)', () => {
    savePositions([{ id: 'n1', position: { x: 1, y: 1 } }], PID_A);
    savePositions([{ id: 'n9', position: { x: 9, y: 9 } }], PID_B);
    expect(loadPositions(PID_A)).toEqual({ n1: { x: 1, y: 1 } });
    expect(loadPositions(PID_B)).toEqual({ n9: { x: 9, y: 9 } });
  });

  it('returns {} for an unknown pipelineId', () => {
    expect(loadPositions('never-saved')).toEqual({});
  });

  it('ignores malformed / non-numeric entries (guard)', () => {
    stub.rawSet(
      'maenifold:pipeline:positions:v1:pipe-A',
      JSON.stringify({
        good: { x: 5, y: 6 },
        nanX: { x: 'nope', y: 6 },
        missingY: { x: 7 },
        nullEntry: null,
        notObject: 42,
      })
    );
    expect(loadPositions(PID_A)).toEqual({ good: { x: 5, y: 6 } });
  });

  it('returns {} for non-JSON raw payloads (try/catch guard)', () => {
    stub.rawSet('maenifold:pipeline:positions:v1:pipe-A', '{not json');
    expect(loadPositions(PID_A)).toEqual({});
  });
});

describe('canvasBridge.viewToFlowEdges (store -> flow edge mapper)', () => {
  it('copies path/equals onto flow edge data only when present and sets edgeType from store type', () => {
    const edges: PipelineEdge[] = [
      { source: 'n1', target: 'n2', type: 'conditional', path: 'out.status', equals: 'done' },
      { source: 'n1', target: 'n3', type: 'success' },
    ];
    const flow = viewToFlowEdges(edges, 'LR');
    expect(flow).toHaveLength(2);

    const cond = flow[0].data as FlowEdgeData;
    expect(cond.edgeType).toBe('conditional');
    expect(cond.path).toBe('out.status');
    expect(cond.equals).toBe('done');

    const plain = flow[1].data as FlowEdgeData;
    expect(plain.edgeType).toBe('success');
    // Absent on the store edge => must NOT be present on flow data.
    expect('path' in plain).toBe(false);
    expect('equals' in plain).toBe(false);
  });

  it('preserves a numeric equals (renders the conditional label end-to-end)', () => {
    const flow = viewToFlowEdges(
      [{ source: 'a', target: 'b', type: 'conditional', path: 'out.count', equals: 0 }],
      'LR'
    );
    const data = flow[0].data as FlowEdgeData;
    // equals === 0 is falsy; the mapper must use `!== undefined`, not truthiness.
    expect(data.equals).toBe(0);
    expect(edgeConditionLabel(data)).toBe('?{out.count}=0');
  });

  it('assigns source/target handles and fan indices for siblings sharing a source', () => {
    const flow = viewToFlowEdges(
      [
        { source: 'n1', target: 'n2', type: 'success' },
        { source: 'n1', target: 'n3', type: 'always' },
      ],
      'LR'
    );
    expect((flow[0].data as FlowEdgeData).fanCount).toBe(2);
    expect((flow[0].data as FlowEdgeData).fanIndex).toBe(0);
    expect((flow[1].data as FlowEdgeData).fanIndex).toBe(1);
    expect(flow[0].sourceHandle).toBe('out-right');
    expect(flow[0].targetHandle).toBe('in-left');
  });
});
