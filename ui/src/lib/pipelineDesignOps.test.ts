import { describe, expect, it } from 'vitest';
import { reactive } from 'vue';
import { duplicateNode, validateDefinition } from './pipelineDesignOps';
import type { PipelineDefinition, ToolNode } from './pipeline-types';

// Binding edges must be backed by explicit `bindings: {from,to}[]` rows. A
// `mapping` value is a supplementary hint and can NEVER satisfy a binding edge
// on its own — these tests pin that contract (red-team Defect 2).
function buildDefinition(
  bindings: { from: string; to: string }[],
  mapping: Record<string, string> | null
): PipelineDefinition {
  const source: ToolNode = {
    type: 'tool',
    id: 'src',
    toolName: 'producer',
    outputSchema: { value: 'string' },
  };
  const target: ToolNode = {
    type: 'tool',
    id: 'dst',
    toolName: 'consumer',
    inputSchema: { value: 'string' },
  };
  return {
    nodes: [source, target],
    edges: [{ source: 'src', target: 'dst', type: 'success', mode: 'binding', bindings, mapping }],
  };
}

describe('validateDefinition binding edges', () => {
  it('rejects a binding edge with no bindings even when mapping is present', () => {
    const issues = validateDefinition(buildDefinition([], { value: 'value' }));
    expect(issues).toHaveLength(1);
    expect(issues[0].kind).toBe('edge');
    expect(issues[0].message).toContain('at least one binding row');
  });

  it('rejects a binding edge whose only binding row is blank', () => {
    const issues = validateDefinition(buildDefinition([{ from: '', to: '' }], null));
    expect(issues).toHaveLength(1);
    expect(issues[0].message).toContain('at least one binding row');
  });

  it('accepts a binding edge with at least one valid binding row', () => {
    const issues = validateDefinition(buildDefinition([{ from: 'value', to: 'value' }], null));
    expect(issues).toHaveLength(0);
  });
});

// Blue-team Defect A: `structuredClone` throws "could not be cloned" on a Vue
// reactive proxy in WebView2. `duplicateNode` must clone reactive store nodes
// without throwing and preserve the discriminant `type` field.
describe('duplicateNode on reactive proxies', () => {
  it('clones a reactive node without throwing and preserves type/fields', () => {
    const node = reactive<ToolNode>({
      type: 'tool',
      id: 'n3',
      toolName: 'MemoryStatus',
      inputSchema: { query: 'string' },
    });
    const clone = duplicateNode(node, 'n3-copy') as ToolNode;
    expect(clone.id).toBe('n3-copy');
    expect(clone.type).toBe('tool');
    expect(clone.toolName).toBe('MemoryStatus');
    expect(clone.inputSchema).toEqual({ query: 'string' });
    // Deep clone: nested object is a distinct reference, not the proxy's.
    clone.inputSchema = { query: 'number' };
    expect(node.inputSchema).toEqual({ query: 'string' });
  });
});

